#ifndef IGNISYEET_HPP
#define IGNISYEET_HPP

/**
 * @file ignisyeet.hpp
 * @brief Main header file for the IgnisYeet rocket flight simulation system
 * 
 * This header provides access to all major components of the IgnisYeet system:
 * - Physics primitives (Vector3D, Quaternion, State)
 * - Rocket vehicle modeling
 * - Environmental models (atmosphere, wind, gravity)
 * - Numerical integration methods
 * - Output management
 * - Simulation engine with Monte Carlo capabilities
 */

// Version information
#define IGNISYEET_VERSION_MAJOR 1
#define IGNISYEET_VERSION_MINOR 0
#define IGNISYEET_VERSION_PATCH 0
#define IGNISYEET_VERSION_STRING "1.0.0"

// Core physics components
#include "physics/vector3d.hpp"
#include "physics/quaternion.hpp"
#include "physics/state.hpp"

// Configuration (must come before other components that use it)
#include "parameter.hpp"

// Rocket system
#include "rocket/vehicle.hpp"

// Environment modeling
#include "environment/atmosphere.hpp"

// Simulation components
#include "simulation/integrator.hpp"
#include "simulation/engine.hpp"

// Output system
#include "output/manager.hpp"

/**
 * @brief Main IgnisYeet namespace
 */
namespace IgnisYeet {

/**
 * @brief Get version information
 */
struct Version {
    static constexpr int major = IGNISYEET_VERSION_MAJOR;
    static constexpr int minor = IGNISYEET_VERSION_MINOR;
    static constexpr int patch = IGNISYEET_VERSION_PATCH;
    static constexpr const char* string = IGNISYEET_VERSION_STRING;
    
    static std::string full_string() {
        return std::string("IgnisYeet v") + IGNISYEET_VERSION_STRING;
    }
};

/**
 * @brief Quick simulation runner for simple use cases
 */
class QuickSim {
public:
    /**
     * @brief Run a simple simulation with default parameters
     * @param config_file Path to TOML configuration file
     * @param output_dir Output directory path
     * @return True if simulation completed successfully
     */
    static bool run_simple(const std::string& config_file, 
                          const std::string& output_dir = "output");
    
    /**
     * @brief Run Monte Carlo simulation
     * @param config_file Path to TOML configuration file
     * @param num_runs Number of Monte Carlo runs
     * @param output_dir Output directory path
     * @return True if simulation completed successfully
     */
    static bool run_monte_carlo(const std::string& config_file, 
                               int num_runs,
                               const std::string& output_dir = "output");
};

/**
 * @brief Utility functions
 */
namespace Utils {
    /**
     * @brief Convert between coordinate systems
     */
    Physics::Vector3D ecef_to_lla(const Physics::Vector3D& ecef);
    Physics::Vector3D lla_to_ecef(const Physics::Vector3D& lla);
    Physics::Vector3D ecef_to_enu(const Physics::Vector3D& ecef, const Physics::Vector3D& ref_lla);
    Physics::Vector3D enu_to_ecef(const Physics::Vector3D& enu, const Physics::Vector3D& ref_lla);
    
    /**
     * @brief Atmospheric calculations
     */
    double altitude_to_pressure(double altitude);
    double pressure_to_altitude(double pressure);
    double mach_number(double velocity, double temperature);
    double reynolds_number(double velocity, double length, double density, double viscosity);
    
    /**
     * @brief Unit conversions
     */
    double deg_to_rad(double degrees);
    double rad_to_deg(double radians);
    double feet_to_meters(double feet);
    double meters_to_feet(double meters);
    double lbf_to_newtons(double lbf);
    double newtons_to_lbf(double newtons);
    
    /**
     * @brief File and directory utilities
     */
    bool create_directory(const std::string& path);
    bool file_exists(const std::string& path);
    std::string get_timestamp_string();
    std::string join_path(const std::string& dir, const std::string& file);
}

/**
 * @brief Constants used throughout the system
 */
namespace PhysicalConstants {
    // Physical constants
    constexpr double EARTH_RADIUS_M = 6371000.0;           // [m]
    constexpr double STANDARD_GRAVITY_MS2 = 9.80665;       // [m/s²]
    constexpr double STANDARD_PRESSURE_PA = 101325.0;      // [Pa]
    constexpr double STANDARD_TEMPERATURE_K = 288.15;      // [K]
    constexpr double STANDARD_DENSITY_KGM3 = 1.225;        // [kg/m³]
    constexpr double GAS_CONSTANT_DRY_AIR_J = 287.0;       // [J/(kg⋅K)]
    constexpr double SPECIFIC_HEAT_RATIO = 1.4;            // [-]
    constexpr double EARTH_ROTATION_RATE_RADS = 7.2921159e-5; // [rad/s]
    
    // Mathematical constants
    constexpr double PI = 3.14159265358979323846;
    constexpr double TWO_PI = 2.0 * PI;
    constexpr double HALF_PI = PI / 2.0;
    constexpr double DEG_TO_RAD = PI / 180.0;
    constexpr double RAD_TO_DEG = 180.0 / PI;
    
    // Conversion factors
    constexpr double FEET_TO_METERS = 0.3048;
    constexpr double METERS_TO_FEET = 1.0 / FEET_TO_METERS;
    constexpr double LBF_TO_NEWTONS = 4.44822;
    constexpr double NEWTONS_TO_LBF = 1.0 / LBF_TO_NEWTONS;
    constexpr double PSI_TO_PASCALS = 6894.76;
    constexpr double PASCALS_TO_PSI = 1.0 / PSI_TO_PASCALS;
}

} // namespace IgnisYeet

// Convenience aliases for commonly used types
namespace IgnisYeet {
    using Vec3 = Physics::Vector3D;
    using Quat = Physics::Quaternion;
    using State = Physics::RigidBodyState;
    using EnvState = Physics::EnvironmentState;
    using Vehicle = Rocket::Vehicle;
    using Env = Environment::Environment;
    using Engine = Simulation::Engine;
    using OutputManager = Output::OutputManager;
}

#endif // IGNISYEET_HPP
