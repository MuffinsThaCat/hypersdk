// Copyright (C) 2024, Ava Labs, Inc. All rights reserved.
// See the file LICENSE for licensing terms.

package runtime

import (
	"context"
	"errors"
	"sync"
	"testing"

	"github.com/ava-labs/avalanchego/ids"
	"github.com/ava-labs/avalanchego/utils/logging"
	"github.com/bytecodealliance/wasmtime-go/v25"
	"github.com/stretchr/testify/require"

	"github.com/ava-labs/hypersdk/codec"
	"github.com/ava-labs/hypersdk/x/contracts/test"
)


func TestEphemeralModuleInstantiation(t *testing.T) {
    require := require.New(t)
    ctx := context.Background()

    rt := newTestRuntime(ctx)
    contract, err := rt.newTestContract("simple")
    require.NoError(err)

    // Call same contract multiple times to verify ephemeral instances
    for i := 0; i < 3; i++ {
        result, err := contract.Call("get_value")
        require.NoError(err)
        require.Equal(uint64(0), into[uint64](result))
    }

    // Verify block stats
    stats := rt.callContext.r.GetBlockStats()
    require.Equal(uint64(3), stats.ContractCalls)
    require.Greater(stats.TotalFuelUsed, uint64(0))
}

func TestEphemeralModuleSystem(t *testing.T) {
    t.Run("Multiple Calls Should Use Cache", func(t *testing.T) {
        require := require.New(t)

        // Create test runtime
        rt := newTestRuntime(context.Background())
        
        // Create and configure mocks
        mockValidator := &mockValidator{}
        mockCache := newMockCache()

        // Set the mocks directly on the runtime
        rt.callContext.r.validator = mockValidator
        rt.callContext.r.cache = mockCache

        // Ensure the test contract is compiled
        err := test.CompileTest("simple")
        require.NoError(err)

        // Create test contract
        contract, err := rt.newTestContract("simple")
        require.NoError(err)

        // Set default fuel for the contract
        contract = contract.WithFuel(1000000)

        // First call - should compile and cache
        result, err := contract.Call("get_value")
        require.NoError(err)
        require.Equal(uint64(0), into[uint64](result))
        require.True(mockValidator.validateCalled, "Validator should be called")
        require.True(mockCache.putCalled, "Cache should store module")

        // Reset flags
        mockValidator.validateCalled = false
        mockCache.getCalled = false
        mockCache.putCalled = false

        // Second call - should use cache
        result, err = contract.Call("get_value")
        require.NoError(err)
        require.Equal(uint64(0), into[uint64](result))
        require.False(mockValidator.validateCalled, "Validator should not be called again")
        require.True(mockCache.getCalled, "Cache should be queried")
        require.False(mockCache.putCalled, "Cache should not store again")
    })

    t.Run("Statistics Tracking", func(t *testing.T) {
        require := require.New(t)

        rt := newTestRuntime(context.Background())
        contract, err := rt.newTestContract("simple")
        require.NoError(err)

        // Get initial stats
        initialStats := rt.callContext.r.GetBlockStats()

        // Make several calls
        numCalls := 3
        for i := 0; i < numCalls; i++ {
            result, err := contract.Call("get_value")
            require.NoError(err)
            require.Equal(uint64(0), into[uint64](result))
        }

        // Verify stats
        finalStats := rt.callContext.r.GetBlockStats()
        require.Equal(initialStats.ContractCalls+uint64(numCalls), finalStats.ContractCalls)
        require.Greater(finalStats.TotalFuelUsed, initialStats.TotalFuelUsed)
        require.Greater(finalStats.CacheHits, initialStats.CacheHits)
    })

    t.Run("Fuel Management", func(t *testing.T) {
        require := require.New(t)

        rt := newTestRuntime(context.Background())
        contract, err := rt.newTestContract("simple")
        require.NoError(err)

        // Try with minimal fuel
        _, err = contract.WithFuel(1).Call("get_value")
        require.Error(err, "Should fail with insufficient fuel")

        // Try with sufficient fuel
        result, err := contract.WithFuel(1000000).Call("get_value")
        require.NoError(err, "Should succeed with sufficient fuel")
        require.Equal(uint64(0), into[uint64](result))
    })

    t.Run("Memory Isolation", func(t *testing.T) {
        require := require.New(t)

        rt := newTestRuntime(context.Background())
        contract, err := rt.newTestContract("simple")
        require.NoError(err)

        // Run concurrent calls
        var wg sync.WaitGroup
        numConcurrent := 5
        results := make(chan error, numConcurrent)

        for i := 0; i < numConcurrent; i++ {
            wg.Add(1)
            go func() {
                defer wg.Done()
                _, err := contract.Call("get_value")
                results <- err
            }()
        }

        wg.Wait()
        close(results)

        // Check all calls succeeded
        for err := range results {
            require.NoError(err, "Concurrent calls should succeed")
        }
    })

    t.Run("Stats Reset", func(t *testing.T) {
        require := require.New(t)
        
        rt := newTestRuntime(context.Background())
        
        rt.callContext.r.ResetBlockStats()
        finalStats := rt.callContext.r.GetBlockStats()
        require.Equal(uint64(0), finalStats.ContractCalls)
        require.Equal(uint64(0), finalStats.TotalFuelUsed)
    })
}

// Mock validator for testing
type mockValidator struct {
    validateCalled bool
    shouldError    bool
}

func (m *mockValidator) ValidateModule(_ context.Context, _ []byte) error {
	m.validateCalled = true
	if m.shouldError {
		return errors.New("validation failed")
	}
	return nil
}


func TestCustomValidation(t *testing.T) {
    require := require.New(t)
    ctx := context.Background()

    // Test successful validation
    t.Run("Successful Validation", func(t *testing.T) {
        validator := &mockValidator{}
        cfg, err := NewConfigBuilder().
            WithValidator(validator).
            Build()
        require.NoError(err)
        rt := NewRuntime(cfg, logging.NoLog{})

        _, err = rt.WithDefaults(CallInfo{
            State: TestStateManager{
                ContractManager: NewContractStateManager(test.NewTestDB(), []byte{}),
            },
            Fuel: 1000000,
        }).CallContract(ctx, &CallInfo{
            Contract:     codec.CreateAddress(0, ids.GenerateTestID()),
            FunctionName: "get_value",
        })
        
        // Should fail because contract doesn't exist, but validator should be called
        require.Error(err)
        require.True(validator.validateCalled)
    })

    // Test failed validation
    t.Run("Failed Validation", func(t *testing.T) {
        validator := &mockValidator{shouldError: true}
        cfg, err := NewConfigBuilder().
            WithValidator(validator).
            Build()
        require.NoError(err)
        rt := NewRuntime(cfg, logging.NoLog{})

        _, err = rt.WithDefaults(CallInfo{
            State: TestStateManager{
                ContractManager: NewContractStateManager(test.NewTestDB(), []byte{}),
            },
            Fuel: 1000000,
        }).CallContract(ctx, &CallInfo{
            Contract:     codec.CreateAddress(0, ids.GenerateTestID()),
            FunctionName: "get_value",
        })
        
        require.Error(err)
        require.True(validator.validateCalled)
        require.Contains(err.Error(), "validation failed")
    })
}

// Mock cache for testing
type mockCache struct {
    modules map[string]*wasmtime.Module
    getCalled bool
    putCalled bool
}

func newMockCache() *mockCache {
	return &mockCache{
		modules: make(map[string]*wasmtime.Module),
	}
}

func (m *mockCache) GetModule(id string) (*wasmtime.Module, bool) {
	m.getCalled = true
	mod, ok := m.modules[id]
	return mod, ok
}

func (m *mockCache) PutModule(id string, module *wasmtime.Module) {
	m.putCalled = true
	m.modules[id] = module
}

func TestCustomCacheStrategy(t *testing.T) {
    require := require.New(t)
    ctx := context.Background()

    cache := newMockCache()
    cfg, err := NewConfigBuilder().
        WithCache(cache).
        Build()
    require.NoError(err)
    rt := NewRuntime(cfg, logging.NoLog{})

    // Create test contract
    contractID := ids.GenerateTestID()
    contractAddr := codec.CreateAddress(0, contractID)
    state := TestStateManager{
        ContractManager: NewContractStateManager(test.NewTestDB(), []byte{}),
    }

    // Compile and set contract
    err = state.CompileAndSetContract(ContractID(contractID[:]), "simple")
    require.NoError(err)
    err = state.SetAccountContract(ctx, contractAddr, ContractID(contractID[:]))
    require.NoError(err)

    // First call - should put in cache
    _, err = rt.WithDefaults(CallInfo{
        State: state,
        Fuel:  1000000,
    }).CallContract(ctx, &CallInfo{
        Contract:     contractAddr,
        FunctionName: "get_value",
    })
    require.NoError(err)
    require.True(cache.putCalled)

    // Reset flags
    cache.getCalled = false
    cache.putCalled = false

    // Second call - should get from cache
    _, err = rt.WithDefaults(CallInfo{
        State: state,
        Fuel:  1000000,
    }).CallContract(ctx, &CallInfo{
        Contract:     contractAddr,
        FunctionName: "get_value",
    })
    require.NoError(err)
    require.True(cache.getCalled)
    require.False(cache.putCalled) // Shouldn't put again
}

func TestEphemeralInstanceIsolation(t *testing.T) {
    require := require.New(t)
    ctx := context.Background()

    rt := newTestRuntime(ctx)
    contract, err := rt.newTestContract("simple")
    require.NoError(err)

    // Run concurrent calls to test isolation
    results := make(chan error, 3)
    for i := 0; i < 3; i++ {
        go func() {
            _, err := contract.Call("get_value")
            results <- err
        }()
    }

    // Verify all calls succeed independently
    for i := 0; i < 3; i++ {
        require.NoError(<-results)
    }
}

func TestEphemeralInstanceCleanup(t *testing.T) {
    require := require.New(t)
    ctx := context.Background()

    rt := newTestRuntime(ctx)
    contract, err := rt.newTestContract("simple")
    require.NoError(err)

    // Get initial stats
    initialStats := rt.callContext.r.GetBlockStats()

    // Make calls
    for i := 0; i < 3; i++ {
        result, err := contract.Call("get_value")
        require.NoError(err)
        require.Equal(uint64(0), into[uint64](result))
    }

    // Verify stats were updated
    finalStats := rt.callContext.r.GetBlockStats()
    require.Equal(initialStats.ContractCalls+3, finalStats.ContractCalls)
    require.Greater(finalStats.TotalFuelUsed, initialStats.TotalFuelUsed)
}

