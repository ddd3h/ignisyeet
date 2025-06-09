#include "../../include/simulation/engine.hpp"
#include "../../include/parameter.hpp"
#include <iostream>
#include <chrono>
#include <algorithm>
#include <cmath>

namespace IgnisYeet::Simulation {

Engine::Engine(const Parameters& params) 
    : current_time_(0.0)
    , time_step_(params.simulation.time_step)
{
    initialize_components(params);
    setup_initial_state();
}

bool Engine::initialize() {
    try {
        // Reset simulation state
        current_time_ = 0.0;
        setup_initial_state();
        
        // Initialize components
        if (!vehicle_->initialize()) {
            return false;
        }
        
        if (!environment_->initialize()) {
            return false;
        }
        
        if (!output_manager_->initialize()) {
            return false;
        }
        
        report_progress(0.0, "Simulation initialized");
        return true;
        
    } catch (const std::exception& e) {
        std::cerr << "Engine initialization failed: " << e.what() << std::endl;
        return false;
    }
}

SimulationResults Engine::run() {
    SimulationResults results;
    
    if (!initialize()) {
        results.error_message = "Failed to initialize simulation";
        return results;
    }
    
    auto start_time = std::chrono::high_resolution_clock::now();
    
    try {
        const double max_time = config_.max_simulation_time;
        size_t step_count = 0;
        
        report_progress(0.0, "Starting simulation");
        
        while (current_time_ < max_time && !check_termination_conditions(current_state_, current_time_)) {
            // Single simulation step
            if (!step()) {
                results.error_message = "Simulation step failed";
                return results;
            }
            
            // Update statistics
            Physics::EnvironmentState env_state = environment_->get_state(current_state_.position, current_time_);
            update_statistics(current_state_, env_state, results);
            
            // Write output
            if (!write_output_record(current_state_, current_time_)) {
                results.error_message = "Failed to write output";
                return results;
            }
            
            // Progress reporting
            double progress = current_time_ / max_time;
            if (step_count % 100 == 0) {  // Report every 100 steps
                std::string status = "Time: " + std::to_string(current_time_) + "s, Altitude: " + 
                                    std::to_string(current_state_.position.z()) + "m";
                report_progress(progress, status);
            }
            
            step_count++;
        }
        
        // Finalize results
        auto end_time = std::chrono::high_resolution_clock::now();
        auto duration = std::chrono::duration_cast<std::chrono::milliseconds>(end_time - start_time);
        
        results.success = true;
        results.total_simulation_time = duration.count() / 1000.0;
        results.total_steps = step_count;
        results.landing_position = current_state_.position;
        results.landing_velocity = current_state_.velocity;
        
        report_progress(1.0, "Simulation completed successfully");
        
    } catch (const std::exception& e) {
        results.error_message = e.what();
        report_progress(1.0, "Simulation failed: " + results.error_message);
    }
    
    return results;
}

bool Engine::step() {
    try {
        // Compute state derivatives
        StateDerivative derivatives = compute_derivatives(current_state_, current_time_);
        
        // Integrate state using selected integration method
        Physics::RigidBodyState new_state = integrator_->integrate(
            current_state_, derivatives, time_step_);
        
        // Update state and time
        current_state_ = new_state;
        current_time_ += time_step_;
        
        return true;
        
    } catch (const std::exception& e) {
        std::cerr << "Integration step failed: " << e.what() << std::endl;
        return false;
    }
}

StateDerivative Engine::compute_derivatives(
    const Physics::RigidBodyState& state, double time) const {
    
    StateDerivative derivatives;
    
    // Position derivative is velocity
    derivatives.position_dot = state.velocity;
    
    // Compute forces and moments
    Physics::ForcesMoments forces_moments = compute_forces_moments(state, time);
    
    // Acceleration from Newton's second law: a = F/m
    derivatives.velocity_dot = forces_moments.force / state.mass;
    
    // Angular acceleration from Euler's equation: α = I^-1 * (τ - ω × (I*ω))
    Physics::Vector3D angular_momentum = state.inertia_tensor * state.angular_velocity;
    Physics::Vector3D gyroscopic = state.angular_velocity.cross(angular_momentum);
    Physics::Vector3D angular_accel = state.inertia_tensor_inverse * (forces_moments.moment - gyroscopic);
    derivatives.angular_velocity_dot = angular_accel;
    
    // Quaternion derivative from angular velocity
    derivatives.orientation_dot = Physics::Quaternion::from_angular_velocity(
        state.angular_velocity, state.orientation);
    
    // Mass flow rate (propellant consumption)
    derivatives.mass_dot = -vehicle_->get_mass_flow_rate(time);
    
    return derivatives;
}

Physics::ForcesMoments Engine::compute_forces_moments(
    const Physics::RigidBodyState& state, double time) const {
    
    Physics::ForcesMoments total;
    
    // Get environment state (traditional environment)
    Physics::EnvironmentState env_state = environment_->get_state(state.position, time);
    
    // Get atmosphere state from new atmosphere model
    auto atm_state = atmosphere_model_->calculate_atmosphere(state.position, time);
    
    // Update environment state with atmosphere model data
    env_state.air_density = atm_state.density;
    env_state.air_temperature = atm_state.temperature;
    env_state.air_pressure = atm_state.pressure;
    env_state.wind_velocity = atm_state.wind_velocity;
    
    // Gravity force using new gravity model
    Physics::Vector3D gravity_acceleration = gravity_model_->calculate_gravity(state.position, state.velocity);
    Physics::Vector3D gravity_force = gravity_acceleration * state.mass;
    total.force += gravity_force;
    
    // Aerodynamic forces and moments (now using atmosphere model data)
    if (state.velocity.magnitude() > 0.01) {  // Only if moving
        auto aero = vehicle_->compute_aerodynamics(state, env_state);
        total.force += aero.force;
        total.moment += aero.moment;
    }
    
    // Propulsion forces and moments
    auto propulsion = vehicle_->compute_propulsion(state, time);
    total.force += propulsion.force;
    total.moment += propulsion.moment;
    
    return total;
}

bool Engine::check_termination_conditions(
    const Physics::RigidBodyState& state, double time) const {
    
    // Ground impact (Z < 0 for typical coordinate system)
    if (state.position.z() < 0.0) {
        return true;
    }
    
    // Maximum altitude reached and descending
    if (state.position.z() > config_.max_altitude) {
        return true;
    }
    
    // Maximum velocity exceeded (safety check)
    if (state.velocity.magnitude() > config_.max_velocity) {
        return true;
    }
    
    return false;
}

void Engine::update_statistics(
    const Physics::RigidBodyState& state, 
    const Physics::EnvironmentState& env_state,
    SimulationResults& results) const {
    
    double altitude = state.position.z();
    double velocity = state.velocity.magnitude();
    double acceleration = compute_derivatives(state, current_time_).velocity_dot.magnitude();
    double mach = velocity / env_state.speed_of_sound;
    
    // Update maximums
    results.max_altitude = std::max(results.max_altitude, altitude);
    results.max_velocity = std::max(results.max_velocity, velocity);
    results.max_acceleration = std::max(results.max_acceleration, acceleration);
    results.max_mach = std::max(results.max_mach, mach);
    
    // Detect events
    if (results.burnout_time == 0.0 && vehicle_->get_thrust(current_time_) <= 0.0) {
        results.burnout_time = current_time_;
    }
    
    if (results.apogee_time == 0.0 && state.velocity.z() <= 0.0 && altitude > 100.0) {
        results.apogee_time = current_time_;
    }
}

bool Engine::write_output_record(
    const Physics::RigidBodyState& state, double time) {
    
    try {
        // Create output record
        Output::OutputRecord record;
        record.time = time;
        record.position = state.position;
        record.velocity = state.velocity;
        record.orientation = state.orientation;
        record.angular_velocity = state.angular_velocity;
        record.mass = state.mass;
        
        // Add derived quantities
        Physics::EnvironmentState env_state = environment_->get_state(state.position, time);
        record.altitude = state.position.z();
        record.velocity_magnitude = state.velocity.magnitude();
        record.mach_number = record.velocity_magnitude / env_state.speed_of_sound;
        
        // Write to output manager
        return output_manager_->write_record(record);
        
    } catch (const std::exception& e) {
        std::cerr << "Failed to write output record: " << e.what() << std::endl;
        return false;
    }
}

void Engine::reset() {
    current_time_ = 0.0;
    setup_initial_state();
    
    if (vehicle_) vehicle_->reset();
    if (environment_) environment_->reset();
    if (output_manager_) output_manager_->reset();
}

void Engine::initialize_components(const Parameters& params) {
    // Store configuration
    config_.max_simulation_time = params.simulation.max_time;
    config_.max_altitude = params.simulation.max_altitude;
    config_.max_velocity = params.simulation.max_velocity;
    
    // Create components (placeholder implementations will be created in subsequent tasks)
    vehicle_ = std::make_unique<Rocket::Vehicle>(params);
    environment_ = std::make_unique<Environment::Environment>(params);
    
    // Create gravity model based on level selection
    create_gravity_model(params);
    
    // Create atmosphere model based on level selection
    create_atmosphere_model(params);
    
    // Create integrator based on method selection
    switch (params.simulation.integration_method) {
        case IntegrationMethod::EULER:
            integrator_ = std::make_unique<EulerIntegrator>();
            break;
        case IntegrationMethod::LEAPFROG:
            integrator_ = std::make_unique<LeapfrogIntegrator>();
            break;
        case IntegrationMethod::RK4:
            integrator_ = std::make_unique<RK4Integrator>();
            break;
        case IntegrationMethod::ADAPTIVE_RK45:
            integrator_ = std::make_unique<AdaptiveRK45Integrator>();
            break;
        default:
            integrator_ = std::make_unique<LeapfrogIntegrator>();
    }
    
    output_manager_ = std::make_unique<Output::OutputManager>(params);
}

void Engine::create_gravity_model(const Parameters& params) {
    // Extract gravity parameters from environment configuration
    int gravity_level = params.environment.gravity.level;
    double g0 = params.environment.gravity.g0;
    double earth_radius = params.environment.gravity.earth_radius;
    double rotation_rate = params.environment.gravity.earth_rotation_rate;
    
    switch (gravity_level) {
        case 1:
            // Level 1: Uniform gravity
            gravity_model_ = std::make_unique<ignis::UniformGravity>(g0);
            break;
            
        case 2:
            // Level 2: Altitude-dependent gravity
            gravity_model_ = std::make_unique<ignis::AltitudeDependentGravity>(g0, earth_radius);
            break;
            
        case 3:
            // Level 3: Rotating Earth with Coriolis effects
            gravity_model_ = std::make_unique<ignis::RotatingEarthGravity>(g0, earth_radius, rotation_rate);
            break;
            
        default:
            // Default to Level 1 if invalid level specified
            std::cerr << "Warning: Invalid gravity level " << gravity_level 
                      << ". Defaulting to Level 1 (uniform gravity)." << std::endl;
            gravity_model_ = std::make_unique<ignis::UniformGravity>(g0);
            break;
    }
    
    std::cout << "Initialized gravity model: " << gravity_model_->get_description() << std::endl;
}

void Engine::create_atmosphere_model(const Parameters& params) {
    // Extract atmosphere parameters from environment configuration
    int atmosphere_level = params.environment.atmosphere.level;
    
    switch (atmosphere_level) {
        case 1: {
            // Level 1: Simple constant atmosphere
            double density = params.environment.atmosphere.simple.density;
            double temperature = params.environment.atmosphere.simple.temperature;
            double pressure = params.environment.atmosphere.simple.pressure;
            
            atmosphere_model_ = std::make_unique<ignis::SimpleAtmosphere>(density, temperature, pressure);
            break;
        }
        
        case 2: {
            // Level 2: ISA Standard Atmosphere
            Physics::Vector3D wind_velocity(0, 0, 0);
            if (params.environment.atmosphere.wind.velocity.size() >= 3) {
                wind_velocity = Physics::Vector3D(
                    params.environment.atmosphere.wind.velocity[0],
                    params.environment.atmosphere.wind.velocity[1],
                    params.environment.atmosphere.wind.velocity[2]
                );
            }
            
            atmosphere_model_ = std::make_unique<ignis::StandardAtmosphere>(wind_velocity);
            break;
        }
        
        case 3: {
            // Level 3: Dynamic Atmosphere
            Physics::Vector3D wind_velocity(0, 0, 0);
            if (params.environment.atmosphere.wind.velocity.size() >= 3) {
                wind_velocity = Physics::Vector3D(
                    params.environment.atmosphere.wind.velocity[0],
                    params.environment.atmosphere.wind.velocity[1],
                    params.environment.atmosphere.wind.velocity[2]
                );
            }
            
            double gust_intensity = params.environment.atmosphere.gust.intensity;
            double gust_frequency = params.environment.atmosphere.gust.frequency;
            
            auto dynamic_atm = std::make_unique<ignis::DynamicAtmosphere>(
                wind_velocity, gust_intensity, gust_frequency
            );
            
            // Set turbulence scale if available
            if (params.environment.atmosphere.gust.turbulence_scale.size() >= 3) {
                Physics::Vector3D turbulence_scale(
                    params.environment.atmosphere.gust.turbulence_scale[0],
                    params.environment.atmosphere.gust.turbulence_scale[1],
                    params.environment.atmosphere.gust.turbulence_scale[2]
                );
                dynamic_atm->set_turbulence_scale(turbulence_scale);
            }
            
            atmosphere_model_ = std::move(dynamic_atm);
            break;
        }
        
        default:
            // Default to Level 2 if invalid level specified
            std::cerr << "Warning: Invalid atmosphere level " << atmosphere_level 
                      << ". Defaulting to Level 2 (Standard ISA)." << std::endl;
            atmosphere_model_ = std::make_unique<ignis::StandardAtmosphere>();
            break;
    }
    
    std::cout << "Initialized atmosphere model: Level " << atmosphere_model_->get_level() 
              << " (" << atmosphere_model_->get_name() << ")" << std::endl;
}

void Engine::setup_initial_state() {
    // This will be initialized from parameters
    current_state_ = Physics::RigidBodyState{
        .position = Physics::Vector3D(0.0, 0.0, 0.0),
        .velocity = Physics::Vector3D(0.0, 0.0, 0.0),
        .orientation = Physics::Quaternion::identity(),
        .angular_velocity = Physics::Vector3D::zero(),
        .mass = 100.0,  // Will be set from parameters
        .inertia_tensor = Physics::Matrix3x3::identity(),
        .inertia_tensor_inverse = Physics::Matrix3x3::identity()
    };
}

void Engine::report_progress(double progress, const std::string& status) {
    if (progress_callback_) {
        progress_callback_(progress, status);
    }
}

} // namespace IgnisYeet::Simulation
