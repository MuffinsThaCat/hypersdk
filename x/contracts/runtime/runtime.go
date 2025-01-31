// Copyright (C) 2024, Ava Labs, Inc. All rights reserved.
// See the file LICENSE for licensing terms.

package runtime

import (
	"context"
	"reflect"
	"sync/atomic"

	"github.com/ava-labs/avalanchego/cache"
	"github.com/ava-labs/avalanchego/utils/logging"
	"github.com/bytecodealliance/wasmtime-go/v25"

	"github.com/ava-labs/hypersdk/codec"
	"github.com/ava-labs/hypersdk/state"
)

type WasmRuntime struct {
    log    logging.Logger
    engine *wasmtime.Engine
    cfg    *Config

    contractCache cache.Cacher[string, *wasmtime.Module]
    callerInfo   map[uintptr]*CallInfo
    linker       *wasmtime.Linker

    // New fields for ephemeral support
    blockStats BlockStats
    validator  ModuleValidator
    cache      CacheStrategy
}

type StateManager interface {
    BalanceManager
    ContractManager
}

type BalanceManager interface {
    GetBalance(ctx context.Context, address codec.Address) (uint64, error)
    TransferBalance(ctx context.Context, from codec.Address, to codec.Address, amount uint64) error
}

type ContractManager interface {
    // GetContractState returns the state of the contract at the given address.
    GetContractState(address codec.Address) state.Mutable
    // GetAccountContract returns the contract ID associated with the given account.
    // An account represents a specific instance of a contract.
    GetAccountContract(ctx context.Context, account codec.Address) (ContractID, error)
    // GetContractBytes returns the compiled WASM bytes of the contract with the given ID.
    GetContractBytes(ctx context.Context, contractID ContractID) ([]byte, error)
    // NewAccountWithContract creates a new account that represents a specific instance of a contract.
    NewAccountWithContract(ctx context.Context, contractID ContractID, accountCreationData []byte) (codec.Address, error)
    // SetAccountContract associates the given contract ID with the given account.
    SetAccountContract(ctx context.Context, account codec.Address, contractID ContractID) error
    // SetContractBytes stores the compiled WASM bytes of the contract with the given ID.
    SetContractBytes(ctx context.Context, contractID ContractID, contractBytes []byte) error
}

func NewRuntime(
    cfg *Config,
    log logging.Logger,
) *WasmRuntime {
    hostImports := NewImports()

    runtime := &WasmRuntime{
        log:        log,
        cfg:        cfg,
        engine:     wasmtime.NewEngineWithConfig(cfg.wasmConfig),
        callerInfo: map[uintptr]*CallInfo{},
        contractCache: cache.NewSizedLRU(cfg.ContractCacheSize, func(id string, mod *wasmtime.Module) int {
            bytes, err := mod.Serialize()
            if err != nil {
                panic(err)
            }
            return len(id) + len(bytes)
        }),
        validator: cfg.Validator,
        cache:     cfg.Cache,
    }

    hostImports.AddModule(NewLogModule())
    hostImports.AddModule(NewBalanceModule())
    hostImports.AddModule(NewStateAccessModule())
    hostImports.AddModule(NewContractModule(runtime))

    linker, err := hostImports.createLinker(runtime)
    if err != nil {
        panic(err)
    }

    runtime.linker = linker

    return runtime
}

func (r *WasmRuntime) WithDefaults(callInfo CallInfo) CallContext {
    return NewCallContext(r, callInfo)
}

func (r *WasmRuntime) getModule(ctx context.Context, callInfo *CallInfo, id []byte) (*wasmtime.Module, error) {
    // Try custom cache strategy first
    if r.cache != nil {
        if mod, ok := r.cache.GetModule(string(id)); ok {
            atomic.AddUint64(&r.blockStats.CacheHits, 1)
            return mod, nil
        }
    }

    // Try default cache
    if mod, ok := r.contractCache.Get(string(id)); ok {
        atomic.AddUint64(&r.blockStats.CacheHits, 1)
        return mod, nil
    }

    contractBytes, err := callInfo.State.GetContractBytes(ctx, id)
    if err != nil {
        return nil, err
    }

    // Validate if configured
    if r.validator != nil {
        if err := r.validator.ValidateModule(ctx, contractBytes); err != nil {
            return nil, err
        }
    }

    mod, err := wasmtime.NewModule(r.engine, contractBytes)
    if err != nil {
        return nil, err
    }

    // Cache the module
    if r.cache != nil {
        r.cache.PutModule(string(id), mod)
    }
    r.contractCache.Put(string(id), mod)

    return mod, nil
}

func (r *WasmRuntime) CallContract(ctx context.Context, callInfo *CallInfo) ([]byte, error) {
    contractID, err := callInfo.State.GetAccountContract(ctx, callInfo.Contract)
    if err != nil {
        return nil, err
    }

    contractModule, err := r.getModule(ctx, callInfo, contractID)
    if err != nil {
        return nil, err
    }

    // Create new ephemeral instance
    instance, err := createEphemeralInstance(
        r.engine,
        r.linker,
        contractModule,
        callInfo.Fuel,
    )
    if err != nil {
        return nil, err
    }
    defer instance.Close()

    // Set up call info
    r.setCallInfo(instance.store, callInfo)
    defer r.deleteCallInfo(instance.store)

    // Execute and update stats
    return instance.Call(ctx, callInfo)
}

func toMapKey(storeLike wasmtime.Storelike) uintptr {
    return reflect.ValueOf(storeLike.Context()).Pointer()
}

func (r *WasmRuntime) setCallInfo(storeLike wasmtime.Storelike, info *CallInfo) {
    r.callerInfo[toMapKey(storeLike)] = info
}

func (r *WasmRuntime) getCallInfo(storeLike wasmtime.Storelike) *CallInfo {
    return r.callerInfo[toMapKey(storeLike)]
}

func (r *WasmRuntime) deleteCallInfo(storeLike wasmtime.Storelike) {
    delete(r.callerInfo, toMapKey(storeLike))
}

// New methods for stats management

func (r *WasmRuntime) GetBlockStats() BlockStats {
    return BlockStats{
        TotalFuelUsed:    atomic.LoadUint64(&r.blockStats.TotalFuelUsed),
        ContractCalls:    atomic.LoadUint64(&r.blockStats.ContractCalls),
        AvgExecutionTime: atomic.LoadUint64(&r.blockStats.AvgExecutionTime),
        CacheHits:        atomic.LoadUint64(&r.blockStats.CacheHits),
    }
}

func (r *WasmRuntime) ResetBlockStats() {
    atomic.StoreUint64(&r.blockStats.TotalFuelUsed, 0)
    atomic.StoreUint64(&r.blockStats.ContractCalls, 0)
    atomic.StoreUint64(&r.blockStats.AvgExecutionTime, 0)
    atomic.StoreUint64(&r.blockStats.CacheHits, 0)
}