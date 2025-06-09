#include "../../include/environment/atmosphere.hpp"
#include "../../include/parameter.hpp"
#include <cmath>
#include <algorithm>
#include <stdexcept>

namespace IgnisYeet::Environment {

Environment::Environment(const Parameters& params) {
    initialize_from_parameters(params);
}

bool Environment::initialize() {
    try {
        atmosphere_.initialize();
        gravity_.initialize();
        wind_.initialize();
        return true;
    } catch (const std::exception& e) {
        return false;
    }
}

void Environment::reset() {
    atmosphere_.reset();
    gravity_.reset();
    wind_.reset();
}

Physics::EnvironmentState Environment::get_state(
    const Physics::Vector3D& position, double time) const {
    
    Physics::EnvironmentState state;
    
    // Get atmospheric properties
    auto atm_state = atmosphere_.get_atmospheric_state(position.z());
    state.air_density = atm_state.air_density;
    state.air_temperature = atm_state.air_temperature;
    state.air_pressure = atm_state.air_pressure;
    state.speed_of_sound = atm_state.speed_of_sound;
    
    // Get wind velocity
    state.wind_velocity = wind_.get_wind_velocity(position, time);
    
    // Get gravity acceleration
    state.gravity_acceleration = gravity_.get_gravity_acceleration(position);
    
    return state;
}

Physics::Vector3D Environment::compute_gravity_force(
    const Physics::Vector3D& position, double mass) const {
    
    Physics::Vector3D gravity_accel = gravity_.get_gravity_acceleration(position);
    return gravity_accel * mass;
}

void Environment::initialize_from_parameters(const Parameters& params) {
    atmosphere_.initialize_from_parameters(params);
    gravity_.initialize_from_parameters(params);
    wind_.initialize_from_parameters(params);
}

// Atmosphere Model Implementation
void AtmosphereModel::initialize_from_parameters(const Parameters& params) {
    physics_level_ = params.environment.physics_level;
    
    // Standard atmosphere parameters
    sea_level_pressure_ = 101325.0; // Pa
    sea_level_temperature_ = 288.15; // K
    sea_level_density_ = 1.225; // kg/m³
    temperature_lapse_rate_ = 0.0065; // K/m
    gas_constant_ = 287.04; // J/(kg·K) for dry air
    gamma_ = 1.4; // Specific heat ratio for air
    
    if (physics_level_ >= 2) {
        // Use ISA (International Standard Atmosphere)
        model_type_ = AtmosphereModelType::ISA;
    } else {
        // Simple exponential model
        model_type_ = AtmosphereModelType::EXPONENTIAL;
        scale_height_ = 8400.0; // m
    }
}

bool AtmosphereModel::initialize() {
    return true;
}

void AtmosphereModel::reset() {
    // Nothing to reset for atmosphere model
}

AtmosphericState AtmosphereModel::get_atmospheric_state(double altitude) const {
    AtmosphericState state;
    
    switch (model_type_) {
        case AtmosphereModelType::EXPONENTIAL:
            state = compute_exponential_atmosphere(altitude);
            break;
        case AtmosphereModelType::ISA:
            state = compute_isa_atmosphere(altitude);
            break;
        default:
            state = compute_exponential_atmosphere(altitude);
    }
    
    return state;
}

AtmosphericState AtmosphereModel::compute_exponential_atmosphere(double altitude) const {
    AtmosphericState state;
    
    // Simple exponential atmosphere model
    double altitude_km = altitude / 1000.0;
    
    // Exponential decay
    double exp_factor = std::exp(-altitude / scale_height_);
    
    state.air_density = sea_level_density_ * exp_factor;
    state.air_pressure = sea_level_pressure_ * exp_factor;
    state.air_temperature = sea_level_temperature_ - temperature_lapse_rate_ * altitude;
    
    // Keep temperature above absolute minimum
    state.air_temperature = std::max(state.air_temperature, 200.0);
    
    // Speed of sound from temperature
    state.speed_of_sound = std::sqrt(gamma_ * gas_constant_ * state.air_temperature);
    
    return state;
}

AtmosphericState AtmosphereModel::compute_isa_atmosphere(double altitude) const {
    AtmosphericState state;
    
    // International Standard Atmosphere (ISA) model
    // Simplified implementation for troposphere (0-11km)
    
    if (altitude <= 11000.0) {
        // Troposphere
        state.air_temperature = sea_level_temperature_ - temperature_lapse_rate_ * altitude;
        
        double temp_ratio = state.air_temperature / sea_level_temperature_;
        double pressure_exp = 9.80665 / (gas_constant_ * temperature_lapse_rate_);
        
        state.air_pressure = sea_level_pressure_ * std::pow(temp_ratio, pressure_exp);
        state.air_density = state.air_pressure / (gas_constant_ * state.air_temperature);
        
    } else if (altitude <= 20000.0) {
        // Lower stratosphere (constant temperature)
        state.air_temperature = 216.65; // K
        
        // Exponential pressure decay in isothermal layer
        double h_diff = altitude - 11000.0;
        double exp_factor = std::exp(-9.80665 * h_diff / (gas_constant_ * state.air_temperature));
        
        state.air_pressure = 22632.0 * exp_factor; // Pressure at 11km
        state.air_density = state.air_pressure / (gas_constant_ * state.air_temperature);
        
    } else {
        // Higher altitudes - use exponential approximation
        double scale_height = gas_constant_ * 216.65 / 9.80665;
        double exp_factor = std::exp(-(altitude - 20000.0) / scale_height);
        
        state.air_temperature = 216.65;
        state.air_pressure = 5474.9 * exp_factor; // Pressure at 20km
        state.air_density = state.air_pressure / (gas_constant_ * state.air_temperature);
    }
    
    // Speed of sound
    state.speed_of_sound = std::sqrt(gamma_ * gas_constant_ * state.air_temperature);
    
    return state;
}

// Gravity Model Implementation
void GravityModel::initialize_from_parameters(const Parameters& params) {
    physics_level_ = params.environment.physics_level;
    
    // Earth parameters
    earth_radius_ = 6371000.0; // m
    earth_mass_ = 5.972e24; // kg
    gravitational_constant_ = 6.67430e-11; // m³/(kg·s²)
    standard_gravity_ = 9.80665; // m/s²
    
    // Earth rotation for Coriolis effects (Level 3)
    earth_rotation_rate_ = 7.2921159e-5; // rad/s
    
    if (physics_level_ >= 2) {
        model_type_ = GravityModelType::ALTITUDE_DEPENDENT;
    } else {
        model_type_ = GravityModelType::CONSTANT;
    }
}

bool GravityModel::initialize() {
    return true;
}

void GravityModel::reset() {
    // Nothing to reset for gravity model
}

Physics::Vector3D GravityModel::get_gravity_acceleration(
    const Physics::Vector3D& position) const {
    
    switch (model_type_) {
        case GravityModelType::CONSTANT:
            return Physics::Vector3D(0.0, 0.0, -standard_gravity_);
            
        case GravityModelType::ALTITUDE_DEPENDENT:
            return compute_altitude_dependent_gravity(position);
            
        case GravityModelType::SPHERICAL_EARTH:
            return compute_spherical_earth_gravity(position);
            
        default:
            return Physics::Vector3D(0.0, 0.0, -standard_gravity_);
    }
}

Physics::Vector3D GravityModel::compute_altitude_dependent_gravity(
    const Physics::Vector3D& position) const {
    
    double altitude = position.z();
    double distance_from_center = earth_radius_ + altitude;
    
    // Newton's law of gravitation with altitude dependence
    double gravity_magnitude = gravitational_constant_ * earth_mass_ / 
                              (distance_from_center * distance_from_center);
    
    return Physics::Vector3D(0.0, 0.0, -gravity_magnitude);
}

Physics::Vector3D GravityModel::compute_spherical_earth_gravity(
    const Physics::Vector3D& position) const {
    
    // Full 3D gravitational field
    double distance = position.magnitude() + earth_radius_;
    
    if (distance < earth_radius_) {
        // Inside Earth (shouldn't happen for rockets)
        return Physics::Vector3D(0.0, 0.0, -standard_gravity_);
    }
    
    double gravity_magnitude = gravitational_constant_ * earth_mass_ / (distance * distance);
    
    // Gravity points toward Earth center
    Physics::Vector3D gravity_direction = -position.normalized();
    
    return gravity_direction * gravity_magnitude;
}

// Wind Model Implementation
void WindModel::initialize_from_parameters(const Parameters& params) {
    physics_level_ = params.environment.physics_level;
    
    // Constant wind components
    constant_wind_ = Physics::Vector3D(
        params.environment.wind_velocity_x,
        params.environment.wind_velocity_y,
        params.environment.wind_velocity_z
    );
    
    // Wind model type based on physics level
    if (physics_level_ >= 2) {
        model_type_ = WindModelType::ALTITUDE_DEPENDENT;
        
        // Wind profile parameters
        reference_altitude_ = 10.0; // m
        wind_shear_exponent_ = 0.14; // Typical value for open terrain
        turbulence_intensity_ = 0.1; // 10% turbulence
        
    } else {
        model_type_ = WindModelType::CONSTANT;
    }
}

bool WindModel::initialize() {
    // Initialize random number generator for turbulence
    random_generator_.seed(std::chrono::steady_clock::now().time_since_epoch().count());
    return true;
}

void WindModel::reset() {
    // Re-seed random generator
    random_generator_.seed(std::chrono::steady_clock::now().time_since_epoch().count());
}

Physics::Vector3D WindModel::get_wind_velocity(
    const Physics::Vector3D& position, double time) const {
    
    switch (model_type_) {
        case WindModelType::CONSTANT:
            return constant_wind_;
            
        case WindModelType::ALTITUDE_DEPENDENT:
            return compute_altitude_dependent_wind(position, time);
            
        case WindModelType::TURBULENT:
            return compute_turbulent_wind(position, time);
            
        default:
            return constant_wind_;
    }
}

Physics::Vector3D WindModel::compute_altitude_dependent_wind(
    const Physics::Vector3D& position, double time) const {
    
    double altitude = position.z();
    
    // Power law wind profile
    double wind_ratio = 1.0;
    if (altitude > reference_altitude_) {
        wind_ratio = std::pow(altitude / reference_altitude_, wind_shear_exponent_);
    }
    
    Physics::Vector3D wind = constant_wind_ * wind_ratio;
    
    // Add simple turbulence if Level 3
    if (physics_level_ >= 3) {
        Physics::Vector3D turbulence = compute_turbulence(position, time);
        wind += turbulence;
    }
    
    return wind;
}

Physics::Vector3D WindModel::compute_turbulent_wind(
    const Physics::Vector3D& position, double time) const {
    
    // Start with altitude-dependent wind
    Physics::Vector3D base_wind = compute_altitude_dependent_wind(position, time);
    
    // Add more sophisticated turbulence
    Physics::Vector3D turbulence = compute_turbulence(position, time);
    
    return base_wind + turbulence;
}

Physics::Vector3D WindModel::compute_turbulence(
    const Physics::Vector3D& position, double time) const {
    
    // Simple turbulence model using random variations
    std::uniform_real_distribution<double> dist(-1.0, 1.0);
    
    // Use position and time as seed modifiers for spatial/temporal correlation
    auto generator = random_generator_;
    generator.seed(static_cast<unsigned int>(
        std::hash<double>{}(position.x() + position.y() + position.z() + time)));
    
    double base_wind_speed = constant_wind_.magnitude();
    double turbulence_magnitude = base_wind_speed * turbulence_intensity_;
    
    Physics::Vector3D turbulence(
        dist(generator) * turbulence_magnitude,
        dist(generator) * turbulence_magnitude,
        dist(generator) * turbulence_magnitude * 0.5  // Less vertical turbulence
    );
    
    return turbulence;
}

} // namespace IgnisYeet::Environment
