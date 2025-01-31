// Copyright (C) 2024, Ava Labs, Inc. All rights reserved.
// See the file LICENSE for licensing terms.

package runtime

import (
	"context"
	"errors"
	"fmt"
	"sync/atomic"
	"time"

	"github.com/bytecodealliance/wasmtime-go/v25"
)

// ModuleValidator allows custom validation of WASM modules before compilation
type ModuleValidator interface {
    // ValidateModule validates the WASM module bytes before compilation
    // Returns error if validation fails
    ValidateModule(ctx context.Context, contractBytes []byte) error
}

// CacheStrategy defines how compiled modules are cached and retrieved
type CacheStrategy interface {
    // GetModule attempts to retrieve a cached compiled module
    // Returns the module and true if found, false if not found
    GetModule(id string) (*wasmtime.Module, bool)
    
    // PutModule caches a compiled module
    PutModule(id string, module *wasmtime.Module)
}

// ExecutionStats tracks runtime statistics for a single execution
type ExecutionStats struct {
    FuelUsed      uint64
    ExecutionTime time.Duration
    CacheHit      bool
}

// BlockStats tracks aggregated statistics for a block
type BlockStats struct {
    TotalFuelUsed    uint64
    ContractCalls    uint64
    AvgExecutionTime uint64 // stored in nanoseconds
    CacheHits        uint64
}

// EphemeralInstance wraps a ContractInstance with execution tracking
type EphemeralInstance struct {
    inst  *wasmtime.Instance
    store *wasmtime.Store
    stats *ExecutionStats
}

// NewEphemeralInstance creates a new ephemeral instance with stats tracking
func NewEphemeralInstance(inst *wasmtime.Instance, store *wasmtime.Store) *EphemeralInstance {
    return &EphemeralInstance{
        inst:  inst,
        store: store,
        stats: &ExecutionStats{
            FuelUsed:      0,
            ExecutionTime: 0,
            CacheHit:      false,
        },
    }
}

// Call executes the contract call and tracks statistics
func (e *EphemeralInstance) Call(ctx context.Context, callInfo *CallInfo) ([]byte, error) {
    if e == nil || e.inst == nil || e.store == nil {
        return nil, errors.New("invalid ephemeral instance")
    }

    if callInfo == nil {
        return nil, errors.New("invalid call info")
    }

    startTime := time.Now()
    startFuel, err := e.store.GetFuel()
    if err != nil {
        return nil, fmt.Errorf("failed to get fuel: %w", err)
    }

    // Create a ContractInstance for the call
    contractInst := &ContractInstance{
        inst:   e.inst,
        store:  e.store,
    }

    // Make the call
    result, err := contractInst.call(ctx, callInfo)
    if err != nil {
        return nil, fmt.Errorf("contract call failed: %w", err)
    }

    // Update stats
    endFuel, _ := e.store.GetFuel()
    e.stats.ExecutionTime = time.Since(startTime)
    e.stats.FuelUsed = startFuel - endFuel

    return result, nil
}

// Close ensures cleanup of resources
func (e *EphemeralInstance) Close() {
    if e.store != nil {
        e.store.Close()
    }
}

// DefaultCacheStrategy provides a basic module caching implementation
type DefaultCacheStrategy struct {
    cache ModuleCache
}

// NewDefaultCacheStrategy creates a new default cache strategy
func NewDefaultCacheStrategy(size int) *DefaultCacheStrategy {
    return &DefaultCacheStrategy{
        cache: NewModuleCache(size),
    }
}

func (d *DefaultCacheStrategy) GetModule(id string) (*wasmtime.Module, bool) {
    return d.cache.Get(id)
}

func (d *DefaultCacheStrategy) PutModule(id string, module *wasmtime.Module) {
    d.cache.Put(id, module)
}

// ModuleCache provides thread-safe caching of compiled modules
type ModuleCache interface {
    Get(id string) (*wasmtime.Module, bool)
    Put(id string, module *wasmtime.Module)
}

// NewModuleCache creates a new module cache with the specified size
func NewModuleCache(size int) ModuleCache {
    // Implementation depends on your caching needs
    // Could use LRU, simple map with mutex, etc.
    return nil // TODO: implement based on needs
}

// UpdateBlockStats updates the block statistics with instance stats
func UpdateBlockStats(blockStats *BlockStats, stats *ExecutionStats) {
    atomic.AddUint64(&blockStats.TotalFuelUsed, stats.FuelUsed)
    atomic.AddUint64(&blockStats.ContractCalls, 1)
    if stats.CacheHit {
        atomic.AddUint64(&blockStats.CacheHits, 1)
    }
    
    // Update average execution time (stored as nanoseconds)
    currentAvg := atomic.LoadUint64(&blockStats.AvgExecutionTime)
    calls := atomic.LoadUint64(&blockStats.ContractCalls)
    newAvg := uint64((time.Duration(currentAvg)*time.Duration(calls-1) + stats.ExecutionTime).Nanoseconds()) / calls
    atomic.StoreUint64(&blockStats.AvgExecutionTime, newAvg)
}

// Helper functions for runtime integration

func createEphemeralInstance(
    engine *wasmtime.Engine,
    linker *wasmtime.Linker,
    module *wasmtime.Module,
    maxFuel uint64,
) (*EphemeralInstance, error) {
    store := wasmtime.NewStore(engine)
    store.SetEpochDeadline(1)
    
    if err := store.SetFuel(maxFuel); err != nil {
        store.Close()
        return nil, err
    }
    
    inst, err := linker.Instantiate(store, module)
    if err != nil {
        store.Close()
        return nil, err
    }

    return &EphemeralInstance{
        inst:  inst,
        store: store,
        stats: &ExecutionStats{
            FuelUsed:      0,
            ExecutionTime: 0,
            CacheHit:      false,
        },
    }, nil
}