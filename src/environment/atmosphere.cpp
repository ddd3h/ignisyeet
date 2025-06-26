#include "../../include/environment/atmosphere.hpp"
#include "../../include/parameter.hpp"
#include <cmath>
#include <algorithm>
#include <stdexcept>
#include <iostream>

namespace IgnisYeet::Environment {

Environment::Environment(const Parameter& params) {
    initialize_from_parameters(params);
}

bool Environment::initialize() {
    try {
        // Initialize atmosphere model
        if (atmosphere_) {
            // atmosphere_->initialize();
        }
        
        // Initialize gravity model  
        if (gravity_) {
            // gravity_->initialize();
        }
        
        // Initialize wind model
        if (wind_) {
            // wind_->initialize();
        }
        
        return true;
    } catch (const std::exception& e) {
        std::cerr << "Environment initialization failed: " << e.what() << std::endl;
        return false;
    }
}

void Environment::reset() {
    // Reset all models to initial state
    if (atmosphere_) {
        // atmosphere_->reset();
    }
    if (gravity_) {
        // gravity_->reset();  
    }
    if (wind_) {
        // wind_->reset();
    }
}

Physics::EnvironmentState Environment::get_state(
    const Physics::Vector3D& position, double time) const {
    return state_at(position, time);
}

void Environment::initialize_from_parameters(const Parameter& params) {
    // Placeholder implementation - will implement component initialization
}

// Atmosphere Model Implementation
void AtmosphereModel::initialize_from_parameters(const Parameter& params) {
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
    
    state.density = sea_level_density_ * exp_factor;
    state.pressure = sea_level_pressure_ * exp_factor;
    state.temperature = sea_level_temperature_ - temperature_lapse_rate_ * altitude;
    
    // Keep temperature above absolute minimum
    state.temperature = std::max(state.temperature, 200.0);
    
    // Speed of sound from temperature
    state.sound_speed = std::sqrt(gamma_ * gas_constant_ * state.temperature);
    
    return state;
}

AtmosphericState AtmosphereModel::compute_isa_atmosphere(double altitude) const {
    AtmosphericState state;
    
    // International Standard Atmosphere (ISA) model
    // Simplified implementation for troposphere (0-11km)
    
    if (altitude <= 11000.0) {
        // Troposphere
        state.temperature = sea_level_temperature_ - temperature_lapse_rate_ * altitude;
        
        double temp_ratio = state.temperature / sea_level_temperature_;
        double pressure_exp = 9.80665 / (gas_constant_ * temperature_lapse_rate_);
        
        state.pressure = sea_level_pressure_ * std::pow(temp_ratio, pressure_exp);
        state.density = state.pressure / (gas_constant_ * state.temperature);
        
    } else if (altitude <= 20000.0) {
        // Lower stratosphere (constant temperature)
        state.temperature = 216.65; // K
        
        // Exponential pressure decay in isothermal layer
        double h_diff = altitude - 11000.0;
        double exp_factor = std::exp(-9.80665 * h_diff / (gas_constant_ * state.temperature));
        
        state.pressure = 22632.0 * exp_factor; // Pressure at 11km
        state.density = state.pressure / (gas_constant_ * state.temperature);
        
    } else {
        // Higher altitudes - use exponential approximation
        double scale_height = gas_constant_ * 216.65 / 9.80665;
        double exp_factor = std::exp(-(altitude - 20000.0) / scale_height);
        
        state.temperature = 216.65;
        state.pressure = 5474.9 * exp_factor; // Pressure at 20km
        state.density = state.pressure / (gas_constant_ * state.temperature);
    }
    
    // Speed of sound
    state.sound_speed = std::sqrt(gamma_ * gas_constant_ * state.temperature);
    
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

// StandardAtmosphereModel Implementation
StandardAtmosphereModel::StandardAtmosphereModel() 
    : sea_level_density_(1.225)      // kg/m³
    , sea_level_pressure_(101325.0)  // Pa
    , sea_level_temperature_(288.15) // K
    , scale_height_(8000.0) {        // m
}

StandardAtmosphereModel::StandardAtmosphereModel(const AtmosphereConfig& config)
    : sea_level_density_(config.sea_level_density)
    , sea_level_pressure_(config.sea_level_pressure) 
    , sea_level_temperature_(config.sea_level_temperature)
    , scale_height_(config.scale_height) {
}

double StandardAtmosphereModel::density(double altitude) const {
    return sea_level_density_ * std::exp(-altitude / scale_height_);
}

double StandardAtmosphereModel::pressure(double altitude) const {
    return sea_level_pressure_ * std::exp(-altitude / scale_height_);
}

double StandardAtmosphereModel::temperature(double altitude) const {
    // Simple linear decrease with altitude
    const double lapse_rate = 0.0065; // K/m
    return sea_level_temperature_ - lapse_rate * altitude;
}

double StandardAtmosphereModel::sound_speed(double altitude) const {
    const double gamma = 1.4; // Heat capacity ratio for air
    const double R = 287.0;   // Specific gas constant for air [J/(kg·K)]
    double temp = temperature(altitude);
    return std::sqrt(gamma * R * temp);
}

Physics::EnvironmentState StandardAtmosphereModel::environment_at(const Physics::Vector3D& position) const {
    double altitude = position.z();
    Physics::EnvironmentState state;
    
    state.density = density(altitude);
    state.pressure = pressure(altitude);
    state.temperature = temperature(altitude);
    state.sound_speed = sound_speed(altitude);
    state.wind_velocity = Physics::Vector3D(0, 0, 0); // No wind for now
    
    return state;
}

} // namespace IgnisYeet::Environment
