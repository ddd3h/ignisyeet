#ifndef PARAMETER_HPP
#define PARAMETER_HPP

#include "rocket.hpp"
#include "environment.hpp"
#include "output.hpp"
#include "toml.hpp"
#include <string>
#include <vector>
#include <map>

// ============================================================================
// SIMULATION CONFIGURATION STRUCTURES
// ============================================================================

struct SimulationConfig {
    std::string name;
    std::string description;
    double max_time;
    double time_step;
    bool adaptive_step;
    double tolerance;
    double output_interval;
    
    // Physics level (1: basic, 2: intermediate, 3: full)
    int physics_level;
    
    // Termination conditions
    bool terminate_on_ground;
    bool terminate_on_max_time;
    bool terminate_on_apogee_descent;
    bool terminate_on_max_altitude;
    double max_altitude_limit;
    
    // Integration method
    std::string integration_method;  // "euler", "leapfrog", "rk4", "rk45"
    
    // Additional engine configuration fields
    double max_simulation_time;   // Alias for max_time
    double max_altitude;         // Maximum altitude for termination
    double max_velocity;         // Maximum velocity for safety check
};

struct RocketConfig {
    std::string name;
    std::string configuration_type;
    
    // Geometry
    double length;
    double diameter;
    double reference_area;
    
    // Mass properties
    double mass_dry;
    double mass_propellant;
    double mass_total;
    
    // Center of gravity
    double cg_initial;
    double cg_final;
    double cg_propellant;
    
    // Moments of inertia [Ixx, Iyy, Izz]
    std::vector<double> inertia_initial;
    std::vector<double> inertia_final;
    
    // CAD integration (optional)
    struct {
        bool enable;
        std::string stl_file;
        bool auto_calculate_inertia;
        bool auto_calculate_cg;
    } cad;
};

struct AerodynamicsConfig {
    int model_level;  // 1: basic drag, 2: full 6DOF, 3: CFD-enhanced
    
    // Reference geometry
    double reference_area;
    double reference_length;
    double center_of_pressure;
    double nose_cone_angle;
    double fin_area;
    
    // Basic drag
    double cd_constant;
    bool cd_mach_dependent;
    std::string cd_table_file;
    
    // Lift and lateral forces
    bool enable_lift;
    bool enable_lateral;
    double cl_alpha;
    double cy_beta;
    double cn_alpha;  // Normal force coefficient
    
    // Moment coefficients
    double cm_alpha;
    double cn_beta;
    double cl_p;
    
    // Pressure center
    double cp_position;
    
    // Compressibility effects
    bool enable_compressibility;
    double mach_critical;
    double mach_drag_rise;
};

struct PropulsionConfig {
    std::string engine_type;
    
    // Thrust characteristics
    std::string thrust_profile_type;
    std::string thrust_profile_file;
    std::vector<std::pair<double, double>> thrust_curve;  // time-thrust pairs
    double thrust_constant;
    double burn_time;
    
    // Performance parameters
    double isp;            // Specific impulse [s]
    double isp_vacuum;
    double isp_sea_level;
    double throat_area;
    double exit_area;
    double expansion_ratio;
    
    // Chamber conditions
    double chamber_pressure;
    double exit_pressure;
};

struct LaunchConfig {
    // Launch site
    double latitude;
    double longitude;
    double altitude;
    
    // Launch conditions
    double azimuth;
    double elevation;
    double rail_length;
    double rail_exit_velocity;
    
    // Initial conditions
    std::vector<double> initial_velocity;       // [vx, vy, vz]
    std::vector<double> initial_angular_velocity; // [wx, wy, wz]
};

struct GravityConfig {
    int level;                    // 1: uniform, 2: altitude_dependent, 3: rotating_earth
    double g0;                   // Standard gravity acceleration [m/s²]
    double earth_radius;         // Earth radius [m] (for level 2+)
    double earth_rotation_rate;  // Earth rotation rate [rad/s] (for level 3)
    bool enable_coriolis;        // Include Coriolis effect (level 3 only)
    bool enable_centrifugal;     // Include centrifugal effect (level 3 only)
    
    // Missing fields for main_v3.cpp compatibility
    int model_type;
    double surface_gravity;
};

struct AtmosphereConfig {
    int level;  // 1: simple, 2: standard_ISA, 3: dynamic
    
    // Missing fields for main_v3.cpp compatibility
    int model_type;
    double sea_level_pressure;
    double sea_level_temperature;
    double sea_level_density;
    double scale_height;
    double temperature_lapse_rate;
    
    // Level 1: Simple atmosphere
    struct {
        double density;      // [kg/m³]
        double temperature;  // [K]
        double pressure;     // [Pa]
    } simple;
    
    // Level 2+: Wind settings
    struct {
        std::vector<double> velocity;  // [m/s] (East, North, Up)
    } wind;
    
    // Level 3: Dynamic atmosphere
    struct {
        double intensity;                    // Gust intensity [m/s]
        double frequency;                    // Gust frequency [Hz]
        std::vector<double> turbulence_scale; // Turbulence scales [m]
    } gust;
};

struct WindConfig {
    int model_type;
    double ground_speed;
    double ground_direction;
    double altitude_coefficient;
    double turbulence_intensity;
    
    WindConfig() 
        : model_type(1)
        , ground_speed(0.0)
        , ground_direction(0.0)
        , altitude_coefficient(0.0)
        , turbulence_intensity(0.0)
    {}
};

struct EnvironmentConfig {
    GravityConfig gravity;
    AtmosphereConfig atmosphere;
    
    // Model selection
    int atmosphere_model;  // 1: simple, 2: standard_ISA, 3: dynamic
    int gravity_model;     // 1: uniform, 2: altitude_dependent, 3: rotating_earth
    int wind_model;        // 1: none, 2: constant, 3: variable
    
    // Missing field for atmosphere.cpp compatibility
    int physics_level;
    
    // Wind configuration
    struct {
        double ground_speed;
        double ground_direction; 
        double altitude_coefficient;
        double turbulence_intensity;
    } wind;
};

struct RecoveryConfig {
    bool enable_parachute;
    double deployment_altitude;
    double deployment_time;
    double drag_coefficient_chute;
    double chute_area;
    double descent_rate_target;
};

struct OutputFieldsConfig {
    bool time;
    bool position;
    bool velocity;
    bool acceleration;
    bool attitude;
    bool angular_velocity;
    bool forces;
    bool moments;
    bool environment;
    bool propulsion;
};

struct OutputConfig {
    std::string primary_format;
    std::vector<std::string> secondary_formats;
    std::string output_directory;
    std::string filename_prefix;
    bool include_timestamp;
    bool include_config_hash;
    std::string coordinate_system;
    OutputFieldsConfig fields;
    
    // Missing fields for main_v3.cpp and manager.cpp compatibility
    std::string format;
    std::string directory;
    std::string base_filename;
    size_t buffer_size;
    int precision;
    bool write_statistics;
    bool enable_buffering;
    std::string filename;
    std::vector<int> output_formats;  // For output format management
    bool enable_csv;
    bool enable_json;
    bool enable_binary;
    bool enable_hdf5;
};

struct MonteCarloConfig {
    bool enable;
    int num_runs;
    int random_seed;
    // Parameter variations (future implementation)
    std::map<std::string, std::map<std::string, double>> variations;
};

// ============================================================================
// MAIN PARAMETER CLASS
// ============================================================================

class Parameter {
public:
    // Configuration structures
    SimulationConfig simulation;
    RocketConfig rocket_config;
    AerodynamicsConfig aerodynamics;
    PropulsionConfig propulsion;
    LaunchConfig launch;
    EnvironmentConfig environment_config;
    RecoveryConfig recovery;
    OutputConfig output_config;
    MonteCarloConfig monte_carlo;
    
    // Missing field for manager.cpp compatibility
    OutputConfig output;
    
    // Legacy compatibility (will be phased out)
    Rocket rocket;
    Environment environment;
    OutputFormat output_format;
    
    // Constructor and methods
    Parameter(const std::string& filename);
    bool loadParameters();
    bool validateParameters();
    void printConfiguration();
    
private:
    std::string param_file;
    bool loadSimulationConfig(const toml::table& config);
    bool loadRocketConfig(const toml::table& config);
    bool loadAerodynamicsConfig(const toml::table& config);
    bool loadPropulsionConfig(const toml::table& config);
    bool loadLaunchConfig(const toml::table& config);
    bool loadEnvironmentConfig(const toml::table& config);
    bool loadRecoveryConfig(const toml::table& config);
    bool loadOutputConfig(const toml::table& config);
    bool loadMonteCarloConfig(const toml::table& config);
};

#endif // PARAMETER_HPP
