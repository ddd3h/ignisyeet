#include "../include/physics/aerodynamics.hpp"
#include "../include/physics/state.hpp"
#include "../include/physics/vector3d.hpp"
#include "../include/physics/quaternion.hpp"
#include "../include/physics/atmosphere.hpp"
#include "../include/parameter.hpp"
#include <iostream>
#include <iomanip>
#include <cmath>
#include <chrono>

using namespace IgnisYeet::Physics;
using ignis::physics::AerodynamicsModel;
using ignis::physics::AerodynamicsLevel;

void testAerodynamicsLevel1() {
    std::cout << "\n=== Testing Aerodynamics Level 1 (Basic Drag) ===" << std::endl;
    
    // Create mock parameters
    Parameter params("../parameter.toml");
    params.loadParameters();
    
    // Initialize aerodynamics model
    AerodynamicsModel aero(AerodynamicsLevel::LEVEL_1, params);
    
    // Test state (velocity in x-direction)
    IgnisYeet::Physics::RigidBodyState state;
    state.position = Vector3D(0, 0, 1000);  // 1km altitude
    state.velocity = Vector3D(100, 0, 0);    // 100 m/s velocity
    state.orientation = IgnisYeet::Physics::Quaternion::identity(); // Identity quaternion
    
    // Atmosphere state
    ignis::AtmosphereModel::AtmosphereState atmo_state;
    atmo_state.density = 1.225;      // Sea level density
    atmo_state.temperature = 288.15; // Sea level temperature
    atmo_state.pressure = 101325;    // Sea level pressure
    
    // Compute aerodynamics
    Vector3D force, moment;
    aero.computeAerodynamics(state, atmo_state, force, moment);
    
    std::cout << "Velocity: (" << state.velocity.x() << ", " << state.velocity.y() << ", " << state.velocity.z() << ") m/s" << std::endl;
    std::cout << "Aero Force: (" << force.x() << ", " << force.y() << ", " << force.z() << ") N" << std::endl;
    std::cout << "Aero Moment: (" << moment.x() << ", " << moment.y() << ", " << moment.z() << ") N⋅m" << std::endl;
    
    // Expected: Drag force opposite to velocity direction
    double expected_drag = 0.3 * 0.1 * 0.5 * 1.225 * 100.0 * 100.0; // cd0 * area * 0.5 * rho * v²
    std::cout << "Expected drag magnitude: " << expected_drag << " N" << std::endl;
    std::cout << "Computed drag magnitude: " << force.magnitude() << " N" << std::endl;
    
    // Verify direction (should be opposite to velocity)
    Vector3D drag_direction = force.normalized();
    Vector3D velocity_direction = state.velocity.normalized();
    double dot_product = drag_direction.dot(velocity_direction);
    std::cout << "Drag-velocity dot product: " << dot_product << " (should be ~ -1)" << std::endl;
}

void testAerodynamicsLevel2() {
    std::cout << "\n=== Testing Aerodynamics Level 2 (Mach-dependent + Lift) ===" << std::endl;
    
    Parameter params("../parameter.toml");
    params.loadParameters();
    AerodynamicsModel aero(AerodynamicsLevel::LEVEL_2, params);
    
    // Test different Mach numbers
    std::vector<double> mach_numbers = {0.3, 0.8, 1.2, 2.0};
    
    for (double mach : mach_numbers) {
        std::cout << "\n--- Mach " << mach << " ---" << std::endl;
        
        double sound_speed = 343.0; // m/s
        double velocity_magnitude = mach * sound_speed;
        
        IgnisYeet::Physics::RigidBodyState state;
        state.position = Vector3D(0, 0, 5000);  // 5km altitude
        state.velocity = Vector3D(velocity_magnitude, 0, 0);
        state.orientation = IgnisYeet::Physics::Quaternion(1, 0, 0, 0);
        
        // Add some angle of attack by rotating attitude
        double alpha = 5.0 * M_PI / 180.0; // 5 degrees AOA
        IgnisYeet::Physics::Quaternion rotation = IgnisYeet::Physics::Quaternion::from_axis_angle(Vector3D(0, 1, 0), alpha);
        state.orientation = rotation;
        
        ignis::AtmosphereModel::AtmosphereState atmo_state;
        atmo_state.density = 0.7364;     // 5km altitude density
        atmo_state.temperature = 255.7;  // 5km temperature
        atmo_state.pressure = 54048;     // 5km pressure
        
        Vector3D force, moment;
        aero.computeAerodynamics(state, atmo_state, force, moment);
        
        std::cout << "Velocity: " << velocity_magnitude << " m/s" << std::endl;
        std::cout << "Aero Force: (" << force.x() << ", " << force.y() << ", " << force.z() << ") N" << std::endl;
        std::cout << "Aero Moment: (" << moment.x() << ", " << moment.y() << ", " << moment.z() << ") N⋅m" << std::endl;
        
        // Check drag vs Mach number trend
        double drag_magnitude = force.magnitude();
        std::cout << "Drag magnitude: " << drag_magnitude << " N" << std::endl;
    }
}

void testAerodynamicsLevel3() {
    std::cout << "\n=== Testing Aerodynamics Level 3 (Full Model) ===" << std::endl;
    
    Parameter params("../parameter.toml");
    params.loadParameters();
    AerodynamicsModel aero(AerodynamicsLevel::LEVEL_3, params);
    
    // Test with sideslip angle
    IgnisYeet::Physics::RigidBodyState state;
    state.position = Vector3D(0, 0, 10000);  // 10km altitude
    state.velocity = Vector3D(300, 50, -20);  // Complex velocity vector
    
    // Create attitude with both angle of attack and sideslip
    double alpha = 10.0 * M_PI / 180.0; // 10 degrees AOA
    double beta = 5.0 * M_PI / 180.0;   // 5 degrees sideslip
    IgnisYeet::Physics::Quaternion pitch_rotation = IgnisYeet::Physics::Quaternion::from_axis_angle(Vector3D(0, 1, 0), alpha);
    IgnisYeet::Physics::Quaternion yaw_rotation = IgnisYeet::Physics::Quaternion::from_axis_angle(Vector3D(0, 0, 1), beta);
    state.orientation = yaw_rotation * pitch_rotation;
    
    // High altitude atmosphere
    ignis::AtmosphereModel::AtmosphereState atmo_state;
    atmo_state.density = 0.4135;     // 10km altitude
    atmo_state.temperature = 223.3;  // 10km temperature
    atmo_state.pressure = 26436;     // 10km pressure
    
    Vector3D force, moment;
    aero.computeAerodynamics(state, atmo_state, force, moment);
    
    std::cout << "Velocity: (" << state.velocity.x() << ", " << state.velocity.y() << ", " << state.velocity.z() << ") m/s" << std::endl;
    std::cout << "Velocity magnitude: " << state.velocity.magnitude() << " m/s" << std::endl;
    std::cout << "Aero Force: (" << force.x() << ", " << force.y() << ", " << force.z() << ") N" << std::endl;
    std::cout << "Aero Moment: (" << moment.x() << ", " << moment.y() << ", " << moment.z() << ") N⋅m" << std::endl;
    
    // Test coefficient computation directly
    double sound_speed = std::sqrt(1.4 * 287.0 * atmo_state.temperature);
    double mach_number = state.velocity.magnitude() / sound_speed;
    double reynolds = 1e6; // Mock Reynolds number
    
    auto coeffs = aero.computeCoefficients(mach_number, reynolds, alpha, beta);
    std::cout << "\nCoefficients:" << std::endl;
    std::cout << "Cd: " << coeffs.cd << std::endl;
    std::cout << "Cl: " << coeffs.cl << std::endl;
    std::cout << "Cs: " << coeffs.cs << std::endl;
    std::cout << "Cm: " << coeffs.cm << std::endl;
    std::cout << "Cn: " << coeffs.cn << std::endl;
    std::cout << "Cr: " << coeffs.cr << std::endl;
}

void testPerformance() {
    std::cout << "\n=== Performance Test ===" << std::endl;
    
    Parameter params("../parameter.toml");
    params.loadParameters();
    AerodynamicsModel aero(AerodynamicsLevel::LEVEL_3, params);
    
    const int num_iterations = 100000;
    
    IgnisYeet::Physics::RigidBodyState state;
    state.position = Vector3D(0, 0, 1000);
    state.velocity = Vector3D(200, 0, 0);
    state.orientation = IgnisYeet::Physics::Quaternion(1, 0, 0, 0);
    
    ignis::AtmosphereModel::AtmosphereState atmo_state;
    atmo_state.density = 1.225;
    atmo_state.temperature = 288.15;
    atmo_state.pressure = 101325;
    
    auto start_time = std::chrono::high_resolution_clock::now();
    
    Vector3D force, moment;
    for (int i = 0; i < num_iterations; ++i) {
        // Slightly vary the state to prevent compiler optimizations
        state.velocity.x() = 200.0 + 0.001 * i;
        aero.computeAerodynamics(state, atmo_state, force, moment);
    }
    
    auto end_time = std::chrono::high_resolution_clock::now();
    auto duration = std::chrono::duration_cast<std::chrono::microseconds>(end_time - start_time);
    
    double avg_time_per_call = static_cast<double>(duration.count()) / num_iterations;
    std::cout << "Average time per aerodynamics computation: " 
              << avg_time_per_call << " microseconds" << std::endl;
    std::cout << "Computations per second: " 
              << static_cast<int>(1e6 / avg_time_per_call) << std::endl;
}

int main() {
    std::cout << "🚀 IgnisYeet Aerodynamics Model Test Suite" << std::endl;
    std::cout << "===========================================" << std::endl;
    
    try {
        testAerodynamicsLevel1();
        testAerodynamicsLevel2();
        testAerodynamicsLevel3();
        testPerformance();
        
        std::cout << "\n✅ All aerodynamics tests completed successfully!" << std::endl;
        
    } catch (const std::exception& e) {
        std::cerr << "\n❌ Test failed with exception: " << e.what() << std::endl;
        return 1;
    }
    
    return 0;
}
