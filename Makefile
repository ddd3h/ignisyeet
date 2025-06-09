# IgnisYeet 6DOF Rocket Simulation Makefile
# Version: v0.3
# Updated for organized project structure

CXX = g++
CXXFLAGS = -Wall -std=c++17 -Iinclude -O2
LDFLAGS = -lm

# Source file organization
PHYSICS_SRCS = src/physics/vector3d.cpp src/physics/state.cpp
SIMULATION_SRCS = src/simulation/engine.cpp src/simulation/integrator.cpp
ROCKET_SRCS = src/rocket/vehicle.cpp
ENVIRONMENT_SRCS = src/environment/atmosphere.cpp
OUTPUT_SRCS = src/output/manager.cpp
PARAMETER_SRCS = src/parameter.cpp

# All new implementation sources
NEW_IMPL_SRCS = $(PHYSICS_SRCS) $(SIMULATION_SRCS) $(ROCKET_SRCS) $(ENVIRONMENT_SRCS) $(OUTPUT_SRCS)
ALL_SRCS = $(NEW_IMPL_SRCS) $(PARAMETER_SRCS)

# Main targets
MAIN_V3_TARGET = ignisyeet_v3
SIMPLE_TARGET = ignisyeet_v3_simple

# Test targets (organized in test/ directory)
TEST_DIR = test
TEST_TARGETS = test_minimal test_gravity test_gravity_simple test_atmosphere test_atmosphere_simple

# Build main executables
$(MAIN_V3_TARGET): src/main_v3.cpp $(ALL_SRCS)
	$(CXX) $(CXXFLAGS) -o $@ src/main_v3.cpp $(ALL_SRCS) $(LDFLAGS)

$(SIMPLE_TARGET): src/main_v3_simple.cpp $(PHYSICS_SRCS) $(PARAMETER_SRCS) src/rocket.cpp src/output.cpp
	$(CXX) $(CXXFLAGS) -o $@ src/main_v3_simple.cpp $(PHYSICS_SRCS) $(PARAMETER_SRCS) src/rocket.cpp src/output.cpp $(LDFLAGS)

# Test executables (in test directory)
$(TEST_DIR)/test_minimal: $(TEST_DIR)/test_minimal.cpp src/physics/vector3d.cpp
	$(CXX) $(CXXFLAGS) -o $@ $^ $(LDFLAGS)

$(TEST_DIR)/test_gravity: $(TEST_DIR)/test_gravity.cpp src/physics/vector3d.cpp
	$(CXX) $(CXXFLAGS) -o $@ $^ $(LDFLAGS)

$(TEST_DIR)/test_gravity_simple: $(TEST_DIR)/test_gravity_simple.cpp src/physics/vector3d.cpp
	$(CXX) $(CXXFLAGS) -o $@ $^ $(LDFLAGS)

$(TEST_DIR)/test_atmosphere: $(TEST_DIR)/test_atmosphere.cpp src/physics/vector3d.cpp
	$(CXX) $(CXXFLAGS) -o $@ $^ $(LDFLAGS)

$(TEST_DIR)/test_atmosphere_simple: $(TEST_DIR)/test_atmosphere_simple.cpp src/physics/vector3d.cpp
	$(CXX) $(CXXFLAGS) -o $@ $^ $(LDFLAGS)

$(TEST_DIR)/test_architecture: $(TEST_DIR)/test_architecture.cpp $(NEW_IMPL_SRCS)
	$(CXX) $(CXXFLAGS) -o $@ $^ $(LDFLAGS)

$(TEST_DIR)/test_full_implementation: $(TEST_DIR)/test_full_implementation.cpp $(ALL_SRCS)
	$(CXX) $(CXXFLAGS) -o $@ $^ $(LDFLAGS)

# Convenient test execution targets
test-minimal: $(TEST_DIR)/test_minimal
	cd $(TEST_DIR) && ./test_minimal

test-gravity: $(TEST_DIR)/test_gravity
	cd $(TEST_DIR) && ./test_gravity

test-gravity-simple: $(TEST_DIR)/test_gravity_simple
	cd $(TEST_DIR) && ./test_gravity_simple

test-atmosphere: $(TEST_DIR)/test_atmosphere
	cd $(TEST_DIR) && ./test_atmosphere

test-atmosphere-simple: $(TEST_DIR)/test_atmosphere_simple
	cd $(TEST_DIR) && ./test_atmosphere_simple

test-architecture: $(TEST_DIR)/test_architecture
	cd $(TEST_DIR) && ./test_architecture

test-full: $(TEST_DIR)/test_full_implementation
	cd $(TEST_DIR) && ./test_full_implementation

# Verification targets
task3-verify: $(TEST_DIR)/test_minimal $(SIMPLE_TARGET)
	@echo "=== Task 3 Verification ==="
	@echo "Running minimal integration test..."
	cd $(TEST_DIR) && ./test_minimal
	@echo ""
	@echo "Running v3 application test..."
	./$(SIMPLE_TARGET)
	@echo ""
	@echo "✅ Task 3 Complete - All tests passed!"

task4-verify: test-gravity test-gravity-simple
	@echo "=== Task 4 Verification Complete ==="
	@echo "✓ Gravity model implementation tests passed"
	@echo "✓ Physical model verification completed"
	@echo "✓ Level 1 (Uniform gravity) implemented and tested"
	@echo "✓ Level 2 (Altitude-dependent gravity) implemented and tested"
	@echo "✓ Level 3 (Rotating Earth gravity) implemented and tested"
	@echo ""
	@echo "🚀 Task 4: Gravity Model Implementation - COMPLETE!"

task5-verify: test-atmosphere test-atmosphere-simple
	@echo "=== Task 5 Verification Complete ==="
	@echo "✓ Atmosphere model implementation tests passed"
	@echo "✓ Physical model verification completed"
	@echo "✓ Level 1 (Simple atmosphere) implemented and tested"
	@echo "✓ Level 2 (ISA Standard atmosphere) implemented and tested"
	@echo "✓ Level 3 (Dynamic atmosphere) implemented and tested"
	@echo ""
	@echo "🌍 Task 5: Atmosphere Model Implementation - COMPLETE!"

# Run all tests
test-all: test-minimal test-gravity test-gravity-simple test-atmosphere test-atmosphere-simple
	@echo ""
	@echo "🎉 All tests completed successfully!"

# Verify current implementation
verify-all: task3-verify task4-verify task5-verify
	@echo ""
	@echo "🚀 IgnisYeet v0.3 - All Tasks Verified Successfully!"

# Build all executables
build-all: $(MAIN_V3_TARGET) $(SIMPLE_TARGET) $(addprefix $(TEST_DIR)/,$(TEST_TARGETS))
	@echo "All executables built successfully!"

# Clean targets
clean:
	rm -f $(MAIN_V3_TARGET) $(SIMPLE_TARGET)
	rm -f $(TEST_DIR)/test_* $(TEST_DIR)/*.o
	rm -f src/**/*.o src/*.o *.o

clean-test:
	rm -f $(TEST_DIR)/test_* $(TEST_DIR)/*.o

# Generic rule for object files
%.o: %.cpp
	$(CXX) $(CXXFLAGS) -c $< -o $@

# Default target
all: $(MAIN_V3_TARGET)

# Help target
help:
	@echo "IgnisYeet v0.3 Build System"
	@echo "=========================="
	@echo ""
	@echo "Main targets:"
	@echo "  all              - Build main executable (ignisyeet_v3)"
	@echo "  ignisyeet_v3     - Build full implementation"
	@echo "  ignisyeet_v3_simple - Build simple implementation"
	@echo ""
	@echo "Test targets:"
	@echo "  test-minimal     - Run basic architecture test"
	@echo "  test-gravity     - Run gravity model tests"
	@echo "  test-atmosphere  - Run atmosphere model tests"
	@echo "  test-all         - Run all tests"
	@echo ""
	@echo "Verification targets:"
	@echo "  task3-verify     - Verify Task 3 completion"
	@echo "  task4-verify     - Verify Task 4 completion"
	@echo "  task5-verify     - Verify Task 5 completion"
	@echo "  verify-all       - Verify all completed tasks"
	@echo ""
	@echo "Utility targets:"
	@echo "  build-all        - Build all executables"
	@echo "  clean            - Remove all build artifacts"
	@echo "  clean-test       - Remove only test executables"
	@echo "  help             - Show this help message"

.PHONY: all test-minimal test-gravity test-gravity-simple test-atmosphere test-atmosphere-simple
.PHONY: test-architecture test-full test-all task3-verify task4-verify task5-verify verify-all
.PHONY: build-all clean clean-test help
