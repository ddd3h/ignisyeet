#ifndef IGNISYEET_SIMULATION_ENGINE_HPP
#define IGNISYEET_SIMULATION_ENGINE_HPP

#include "../physics/state.hpp"
#include "../physics/gravity.hpp"  // Add gravity model support
#include "../physics/atmosphere.hpp"  // Add atmosphere model support
#include "../rocket/vehicle.hpp"
#include "../environment/atmosphere.hpp"
#include "../output/manager.hpp"
#include "../parameter.hpp"  // Added for parameter structures
#include "integrator.hpp"
#include <memory>
#include <functional>
#include <vector>
// #include <random>  // Temporarily disabled
#include <limits>

namespace IgnisYeet::Simulation {

// Use actual parameter types instead of forward declarations
using Parameters = ::Parameter;  // Use the global Parameter class
using SimulationConfig = ::SimulationConfig;
using MonteCarloConfig = ::MonteCarloConfig;

/**
 * @brief Simulation results and statistics
 */
struct SimulationResults {
    bool success;
    std::string error_message;
    
    // Flight events
    double burnout_time;
    double apogee_time;
    double landing_time;
    
    // Maximum values
    double max_altitude;
    double max_velocity;
    double max_acceleration;
    double max_mach;
    
    // Landing conditions
    Physics::Vector3D landing_position;
    Physics::Vector3D landing_velocity;
    
    // Total simulation time
    double total_simulation_time;
    size_t total_steps;
    
    SimulationResults() 
        : success(false)
        , burnout_time(0.0)
        , apogee_time(0.0) 
        , landing_time(0.0)
        , max_altitude(0.0)
        , max_velocity(0.0)
        , max_acceleration(0.0)
        , max_mach(0.0)
        , landing_position(Physics::Vector3D::zero())
        , landing_velocity(Physics::Vector3D::zero())
        , total_simulation_time(0.0)
        , total_steps(0)
    {}
};

/**
 * @brief Engine configuration structure
 */
struct EngineConfig {
    // Simulation parameters
    double max_time;                      // Maximum simulation time [s]
    double dt;                           // Time step [s]
    std::string integrator_type;         // Integrator type
    int physics_level;                   // Physics complexity level
    double output_interval;              // Output interval [s]
    
    // Termination conditions
    bool terminate_on_ground;            // Terminate on ground impact
    bool terminate_on_max_time;          // Terminate on max time
    bool terminate_on_max_altitude;      // Terminate on max altitude
    double max_altitude_limit;           // Max altitude limit [m]
    
    EngineConfig() 
        : max_time(1000.0)
        , dt(0.01)
        , integrator_type("euler")
        , physics_level(1)
        , output_interval(0.1)
        , terminate_on_ground(true)
        , terminate_on_max_time(true)
        , terminate_on_max_altitude(false)
        , max_altitude_limit(100000.0) 
    {}
};

/**
 * @brief Progress callback function type
 */
using ProgressCallback = std::function<void(double progress, const std::string& status)>;

/**
 * @brief Main simulation engine
 */
class Engine {
private:
    // Configuration
    SimulationConfig config_;
    
    // Components
    std::unique_ptr<Rocket::Vehicle> vehicle_;
    std::unique_ptr<Environment::Environment> environment_;
    std::unique_ptr<ignis::GravityModel> gravity_model_;        // Gravity model
    std::unique_ptr<ignis::AtmosphereModel> atmosphere_model_;  // Atmosphere model
    std::unique_ptr<Integrator> integrator_;
    std::unique_ptr<Output::OutputManager> output_manager_;
    
    // Simulation state
    Physics::RigidBodyState current_state_;
    double current_time_;
    double time_step_;
    
    // Callbacks
    ProgressCallback progress_callback_;
    
    // Internal methods
    StateDerivative compute_derivatives(
        const Physics::RigidBodyState& state, double time) const;
    
    Physics::ForcesMoments compute_forces_moments(
        const Physics::RigidBodyState& state, double time) const;
    
    bool check_termination_conditions(
        const Physics::RigidBodyState& state, double time) const;
    
    void update_statistics(
        const Physics::RigidBodyState& state, 
        const Physics::EnvironmentState& env_state,
        SimulationResults& results) const;
    
    bool write_output_record(
        const Physics::RigidBodyState& state, double time);
    
public:
    Engine(const Parameter& params);
    Engine(const SimulationConfig& config, 
           std::shared_ptr<Rocket::Vehicle> vehicle,
           std::shared_ptr<Environment::AtmosphereModel> atmosphere,
           std::shared_ptr<Output::OutputManager> output_manager);
    ~Engine() = default;
    
    // Non-copyable but movable
    Engine(const Engine&) = delete;
    Engine& operator=(const Engine&) = delete;
    Engine(Engine&&) = default;
    Engine& operator=(Engine&&) = default;
    
    // Configuration
    const SimulationConfig& config() const { return config_; }
    
    // Initialize simulation
    bool initialize();
    
    // Run complete simulation
    SimulationResults run();
    
    // Single step simulation (for interactive use)
    bool step();
    
    // Get current state
    const Physics::RigidBodyState& current_state() const { return current_state_; }
    double current_time() const { return current_time_; }
    
    // Progress monitoring
    void set_progress_callback(ProgressCallback callback) {
        progress_callback_ = callback;
    }
    
    // Reset simulation to initial conditions
    void reset();
    
    // Component access
    const Rocket::Vehicle& vehicle() const { return *vehicle_; }
    const Environment::Environment& environment() const { return *environment_; }
    const ignis::GravityModel& gravity_model() const { return *gravity_model_; }          // Gravity model accessor
    const ignis::AtmosphereModel& atmosphere_model() const { return *atmosphere_model_; } // Atmosphere model accessor
    const Integrator& integrator() const { return *integrator_; }
    const Output::OutputManager& output_manager() const { return *output_manager_; }
    
    // Set initial state
    void set_initial_state(const Physics::RigidBodyState& state);
    
    // Get statistics  
    Output::OutputManager::Statistics get_statistics() const;
    
private:
    void initialize_components(const Parameters& params);
    void create_gravity_model(const Parameters& params);      // Gravity model creation method
    void create_atmosphere_model(const Parameters& params);   // Atmosphere model creation method
    void setup_initial_state();
    void report_progress(double progress, const std::string& status);
};

/**
 * @brief Monte Carlo simulation manager
 */
class MonteCarloManager {
private:
    Parameters base_parameters_;
    MonteCarloConfig config_;
    std::vector<SimulationResults> results_;
    
    // Parameter variation
    struct ParameterVariation {
        std::string name;
        double nominal_value;
        double std_deviation;
        double min_value;
        double max_value;
    };
    
    std::vector<ParameterVariation> variations_;
    
    // Random number generation (temporarily disabled)
    // mutable std::mt19937 random_generator_;
    
    Parameters generate_varied_parameters() const;
    void setup_variations();
    
public:
    MonteCarloManager(const Parameters& base_params);
    
    // Run Monte Carlo simulation
    bool run_monte_carlo();
    
    // Results analysis
    struct Statistics {
        struct StatValue {
            double mean;
            double std_dev;
            double min;
            double max;
            double percentile_5;
            double percentile_95;
        };
        
        StatValue max_altitude;
        StatValue landing_distance;
        StatValue landing_velocity;
        StatValue flight_time;
        
        size_t successful_runs;
        size_t total_runs;
        double success_rate;
    };
    
    Statistics compute_statistics() const;
    bool export_results(const std::string& filename) const;
    
    // Dispersion analysis
    struct DispersionAnalysis {
        Physics::Vector3D mean_landing_position;
        Physics::Vector3D landing_std_deviation;
        double cep_50;  // Circular Error Probable (50%)
        double cep_95;  // Circular Error Probable (95%)
        
        std::vector<Physics::Vector3D> landing_positions;
    };
    
    DispersionAnalysis analyze_landing_dispersion() const;
    
    // Progress monitoring
    void set_progress_callback(ProgressCallback callback);
    
private:
    ProgressCallback progress_callback_;
    void report_progress(double progress, const std::string& status);
};

} // namespace IgnisYeet::Simulation

#endif // IGNISYEET_SIMULATION_ENGINE_HPP
