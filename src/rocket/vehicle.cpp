#include "../../include/rocket/vehicle.hpp"
#include "../../include/parameter.hpp"
#include <cmath>
#include <stdexcept>
#include <algorithm>
#include <iostream>

namespace IgnisYeet::Rocket {

Vehicle::Vehicle(const Parameter& params) {
    initialize_from_parameters(params);
}

// VehicleConfig constructor
Vehicle::Vehicle(const VehicleConfig& config) {
    mass_props_.dry_mass = config.dry_mass;
    mass_props_.propellant_mass = config.propellant_mass;
    geometry_.length = config.length;
    geometry_.diameter = config.diameter;
    geometry_.reference_area = M_PI * config.diameter * config.diameter / 4.0;
    
    // Initialize other components
    propulsion_system_.initialize_from_config(config);
    aerodynamics_system_.initialize_from_config(config);
    recovery_system_.initialize_from_config(config);
}

bool Vehicle::initialize() {
    try {
        // Update current mass
        current_mass_ = mass_props_.dry_mass + mass_props_.propellant_mass;
        
        // Initialize propulsion system
        if (propulsion_) {
            // propulsion_->initialize();
        }
        
        // Initialize aerodynamics
        if (aerodynamics_) {
            // aerodynamics_->initialize();
        }
        
        // Initialize recovery
        if (recovery_) {
            // recovery_->initialize();
        }
        
        return true;
    } catch (const std::exception& e) {
        std::cerr << "Vehicle initialization failed: " << e.what() << std::endl;
        return false;
    }
}

void Vehicle::reset() {
    current_mass_ = mass_props_.dry_mass + mass_props_.propellant_mass;
    // Reset all subsystems to initial state
}

double Vehicle::get_mass(double time) const {
    return current_mass_;
}

double Vehicle::get_thrust(double time) const {
    if (propulsion_) {
        return propulsion_->thrust(time);
    }
    return 0.0;
}

double Vehicle::get_mass_flow_rate(double time) const {
    if (propulsion_) {
        return propulsion_->mass_flow_rate(time);
    }
    return 0.0;
}

Physics::ForcesMoments Vehicle::compute_aerodynamics(
    const Physics::RigidBodyState& state,
    const Physics::EnvironmentState& env) const {
    
    Physics::ForcesMoments result;
    
    if (aerodynamics_ && state.velocity.magnitude() > 0.01) {
        // Compute aerodynamic forces using aerodynamics model
        // This is a placeholder - actual implementation depends on aerodynamics model
        double dynamic_pressure = 0.5 * env.air_density * state.velocity.magnitude_squared();
        double drag_force = 0.5 * dynamic_pressure * geometry_.reference_area;  // Simple drag
        
        // Drag force opposes velocity direction
        Physics::Vector3D drag_direction = -state.velocity.normalized();
        result.force = drag_direction * drag_force;
    }
    
    return result;
}

void Vehicle::initialize_from_parameters(const Parameter& params) {
    // Initialize from parameter configuration
    mass_props_.dry_mass = params.rocket_config.mass_dry;
    mass_props_.propellant_mass = params.rocket_config.mass_propellant;
    geometry_.length = params.rocket_config.length;
    geometry_.diameter = params.rocket_config.diameter;
    geometry_.reference_area = params.rocket_config.reference_area;
}

// Propulsion System Implementation
void PropulsionSystem::initialize_from_parameters(const Parameter& params) {
    thrust_curve_.clear();
    
    // Simple constant thrust model for now
    max_thrust_ = params.propulsion.thrust;
    burn_time_ = params.propulsion.burn_time;
    specific_impulse_ = params.propulsion.specific_impulse;
    
    // Create simple thrust curve
    thrust_curve_.push_back({0.0, max_thrust_});
    thrust_curve_.push_back({burn_time_, max_thrust_});
    thrust_curve_.push_back({burn_time_ + 0.1, 0.0});
    
    // Thrust vector (aligned with vehicle axis for now)
    thrust_direction_ = Physics::Vector3D(0.0, 0.0, 1.0);
}

bool PropulsionSystem::initialize() {
    is_burning_ = false;
    total_impulse_delivered_ = 0.0;
    return true;
}

void PropulsionSystem::reset() {
    is_burning_ = false;
    total_impulse_delivered_ = 0.0;
}

double PropulsionSystem::get_thrust(double time) const {
    if (time > burn_time_) {
        return 0.0;
    }
    
    // Linear interpolation in thrust curve
    for (size_t i = 0; i < thrust_curve_.size() - 1; ++i) {
        if (time >= thrust_curve_[i].first && time <= thrust_curve_[i + 1].first) {
            double t_frac = (time - thrust_curve_[i].first) / 
                           (thrust_curve_[i + 1].first - thrust_curve_[i].first);
            return thrust_curve_[i].second + 
                   t_frac * (thrust_curve_[i + 1].second - thrust_curve_[i].second);
        }
    }
    
    return 0.0;
}

double PropulsionSystem::mass_flow_rate(double time) const {
    double thrust = get_thrust(time);
    if (thrust <= 0.0) {
        return 0.0;
    }
    
    // Mass flow rate from rocket equation: dm/dt = F / (Isp * g0)
    const double g0 = 9.80665; // Standard gravity
    return thrust / (specific_impulse_ * g0);
}

Physics::ForcesMoments PropulsionSystem::compute_forces_moments(
    const Physics::RigidBodyState& state,
    double time) const {
    
    Physics::ForcesMoments result;
    
    double thrust = get_thrust(time);
    if (thrust > 0.0) {
        // Thrust force in body frame
        Physics::Vector3D thrust_force_body = thrust_direction_ * thrust;
        
        // Transform to world frame
        result.force = state.orientation.rotate(thrust_force_body);
        
        // Moment due to thrust offset (if any)
        // For now, assume thrust acts through center of mass
        result.moment = Physics::Vector3D::zero();
    }
    
    return result;
}

// Aerodynamics System Implementation
void AerodynamicsSystem::initialize_from_parameters(const Parameters& params) {
    // Basic aerodynamic coefficients
    drag_coefficient_ = params.aerodynamics.drag_coefficient;
    reference_area_ = params.rocket.diameter * params.rocket.diameter * M_PI / 4.0;
    
    // Advanced coefficients (Level 2+)
    if (params.aerodynamics.physics_level >= 2) {
        lift_coefficient_ = params.aerodynamics.lift_coefficient;
        side_force_coefficient_ = params.aerodynamics.side_force_coefficient;
        roll_moment_coefficient_ = params.aerodynamics.roll_moment_coefficient;
        pitch_moment_coefficient_ = params.aerodynamics.pitch_moment_coefficient;
        yaw_moment_coefficient_ = params.aerodynamics.yaw_moment_coefficient;
    }
    
    physics_level_ = params.aerodynamics.physics_level;
}

bool AerodynamicsSystem::initialize() {
    return true;
}

Physics::ForcesMoments AerodynamicsSystem::compute_forces_moments(
    const Physics::RigidBodyState& state,
    const Physics::EnvironmentState& env_state) const {
    
    Physics::ForcesMoments result;
    
    // Relative velocity (accounting for wind)
    Physics::Vector3D relative_velocity = state.velocity - env_state.wind_velocity;
    double velocity_magnitude = relative_velocity.magnitude();
    
    if (velocity_magnitude < 0.1) {
        return result; // No aerodynamics at very low speeds
    }
    
    // Dynamic pressure
    double dynamic_pressure = 0.5 * env_state.air_density * velocity_magnitude * velocity_magnitude;
    
    // Basic drag force (Level 1)
    Physics::Vector3D drag_force = -relative_velocity.normalized() * 
                                   (drag_coefficient_ * reference_area_ * dynamic_pressure);
    result.force += drag_force;
    
    // Advanced aerodynamics (Level 2+)
    if (physics_level_ >= 2) {
        // Angle of attack calculation (simplified)
        Physics::Vector3D body_x = state.orientation.rotate(Physics::Vector3D(1.0, 0.0, 0.0));
        Physics::Vector3D velocity_unit = relative_velocity.normalized();
        double angle_of_attack = std::acos(std::abs(velocity_unit.dot(body_x)));
        
        // Lift and side forces
        if (angle_of_attack > 0.01) { // Small angle threshold
            Physics::Vector3D lift_direction = velocity_unit.cross(body_x).normalized();
            double lift_magnitude = lift_coefficient_ * reference_area_ * dynamic_pressure * angle_of_attack;
            result.force += lift_direction * lift_magnitude;
        }
        
        // Aerodynamic moments (simplified)
        double moment_magnitude = pitch_moment_coefficient_ * reference_area_ * 
                                 dynamic_pressure * angle_of_attack;
        result.moment += Physics::Vector3D(0.0, moment_magnitude, 0.0);
    }
    
    // Compressibility effects (Level 3)
    if (physics_level_ >= 3) {
        double mach_number = velocity_magnitude / env_state.speed_of_sound;
        
        if (mach_number > 0.8) {
            // Simple compressibility correction
            double compressibility_factor = 1.0 + 0.2 * (mach_number - 0.8);
            result.force *= compressibility_factor;
        }
    }
    
    return result;
}

// Recovery System Implementation
void RecoverySystem::initialize_from_parameters(const Parameters& params) {
    deploy_altitude_ = params.recovery.main_chute_deploy_altitude;
    drogue_deploy_altitude_ = params.recovery.drogue_chute_deploy_altitude;
    main_chute_area_ = params.recovery.main_chute_area;
    drogue_chute_area_ = params.recovery.drogue_chute_area;
    main_chute_cd_ = params.recovery.main_chute_drag_coefficient;
    drogue_chute_cd_ = params.recovery.drogue_chute_drag_coefficient;
}

bool RecoverySystem::initialize() {
    state_ = RecoveryState::INACTIVE;
    return true;
}

void RecoverySystem::reset() {
    state_ = RecoveryState::INACTIVE;
}

void RecoverySystem::update(const Physics::RigidBodyState& vehicle_state, double time) {
    double altitude = vehicle_state.position.z();
    
    switch (state_) {
        case RecoveryState::INACTIVE:
            // Check for drogue deployment (apogee or altitude)
            if (vehicle_state.velocity.z() < 0 && altitude < drogue_deploy_altitude_) {
                state_ = RecoveryState::DROGUE_DEPLOYED;
            }
            break;
            
        case RecoveryState::DROGUE_DEPLOYED:
            // Check for main chute deployment
            if (altitude < deploy_altitude_) {
                state_ = RecoveryState::MAIN_DEPLOYED;
            }
            break;
            
        case RecoveryState::MAIN_DEPLOYED:
            // Recovery system is fully deployed
            break;
    }
}

Physics::ForcesMoments RecoverySystem::compute_forces_moments(
    const Physics::RigidBodyState& state,
    const Physics::EnvironmentState& env_state) const {
    
    Physics::ForcesMoments result;
    
    if (state_ == RecoveryState::INACTIVE) {
        return result;
    }
    
    // Parachute drag calculation
    Physics::Vector3D relative_velocity = state.velocity - env_state.wind_velocity;
    double velocity_magnitude = relative_velocity.magnitude();
    
    if (velocity_magnitude > 0.1) {
        double drag_area = 0.0;
        double drag_coefficient = 0.0;
        
        if (state_ == RecoveryState::DROGUE_DEPLOYED) {
            drag_area = drogue_chute_area_;
            drag_coefficient = drogue_chute_cd_;
        } else if (state_ == RecoveryState::MAIN_DEPLOYED) {
            drag_area = main_chute_area_;
            drag_coefficient = main_chute_cd_;
        }
        
        double dynamic_pressure = 0.5 * env_state.air_density * velocity_magnitude * velocity_magnitude;
        Physics::Vector3D drag_force = -relative_velocity.normalized() * 
                                       (drag_coefficient * drag_area * dynamic_pressure);
        result.force = drag_force;
    }
    
    return result;
}

} // namespace IgnisYeet::Rocket
