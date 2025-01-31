// Copyright (C) 2024, Ava Labs, Inc. All rights reserved.
// See the file LICENSE for licensing terms.

package runtime

import "C"

import (
	"context"
	"errors"
	"fmt"

	"github.com/ava-labs/avalanchego/ids"
	"github.com/bytecodealliance/wasmtime-go/v25"

	"github.com/ava-labs/hypersdk/codec"
)

const (
    AllocName  = "alloc"
    MemoryName = "memory"
)

type ContractID []byte

type Context struct {
    Contract  codec.Address
    Actor     codec.Address
    Height    uint64
    Timestamp uint64
    ActionID  ids.ID
}

type CallInfo struct {
    State StateManager
    Actor codec.Address
    FunctionName string
    Contract codec.Address
    Params []byte
    Fuel uint64
    Height uint64
    Timestamp uint64
    ActionID ids.ID
    Value uint64
    inst *ContractInstance
}

type ContractInstance struct {
    inst   *wasmtime.Instance
    store  *wasmtime.Store
    result []byte
}

// Add Close method
func (p *ContractInstance) Close() {
    if p.store != nil {
        p.store.Close()
    }
}

func (p *ContractInstance) call(ctx context.Context, callInfo *CallInfo) ([]byte, error) {
    if p == nil || p.inst == nil || p.store == nil {
        return nil, errors.New("invalid contract instance")
    }

    function := p.inst.GetFunc(p.store, callInfo.FunctionName)
    if function == nil {
        return nil, fmt.Errorf("function %s does not exist", callInfo.FunctionName)
    }

    // Create the contract context
    contractCtx := Context{
        Contract:  callInfo.Contract,
        Actor:     callInfo.Actor,
        Height:    callInfo.Height,
        Timestamp: callInfo.Timestamp,
        ActionID:  callInfo.ActionID,
    }
    
    paramsBytes, err := Serialize(contractCtx)
    if err != nil {
        return nil, fmt.Errorf("failed to serialize context: %w", err)
    }
    paramsBytes = append(paramsBytes, callInfo.Params...)

    // Copy params into store linear memory
    paramsOffset, err := p.writeToMemory(paramsBytes)
    if err != nil {
        return nil, fmt.Errorf("failed to write to memory: %w", err)
    }

    _, err = function.Call(p.store, paramsOffset)
    if err != nil {
        return nil, fmt.Errorf("function call failed: %w", err)
    }

    return p.result, nil
}


func (p *ContractInstance) writeToMemory(data []byte) (int32, error) {
    allocFn := p.inst.GetExport(p.store, AllocName).Func()
    contractMemory := p.inst.GetExport(p.store, MemoryName).Memory()
    dataOffsetIntf, err := allocFn.Call(p.store, int32(len(data)))
    if err != nil {
        return 0, err
    }
    dataOffset := dataOffsetIntf.(int32)
    linearMem := contractMemory.UnsafeData(p.store)
    copy(linearMem[dataOffset:], data)
    return dataOffset, nil
}

func (c *CallInfo) RemainingFuel() uint64 {
    remaining, err := c.inst.store.GetFuel()
    if err != nil {
        return c.Fuel
    }
    return remaining
}

func (c *CallInfo) AddFuel(fuel uint64) {
    remaining, err := c.inst.store.GetFuel()
    if err != nil {
        return
    }
    _ = c.inst.store.SetFuel(remaining + fuel)
}

func (c *CallInfo) ConsumeFuel(fuel uint64) error {
    remaining, err := c.inst.store.GetFuel()
    if err != nil {
        return err
    }
    if remaining < fuel {
        return errors.New("out of fuel")
    }
    return c.inst.store.SetFuel(remaining - fuel)
}
