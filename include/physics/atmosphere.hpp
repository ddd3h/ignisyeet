/**
 * @file atmosphere.hpp
 * @brief Comprehensive atmosphere models for rocket simulation
 * @details 3-level atmosphere model system:
 *          Level 1: Simple constant atmosphere
 *          Level 2: ISA Standard Atmosphere with altitude-dependent properties  
 *          Level 3: Dynamic atmosphere with wind and gust effects
 * 
 * @author IgnisYeet Development Team
 * @date 2025-06-09
 */

#ifndef IGNIS_PHYSICS_ATMOSPHERE_HPP
#define IGNIS_PHYSICS_ATMOSPHERE_HPP

#include <cmath>
#include <vector>
#include <string>
#include <memory>
#include <chrono>
#include "vector3d.hpp"

// Use the Physics namespace for Vector3D
using IgnisYeet::Physics::Vector3D;

namespace ignis {

/**
 * @brief Abstract base class for atmosphere models
 */
class AtmosphereModel {
public:
    /**
     * @brief Atmospheric state at a given position and time
     */
    struct AtmosphereState {
        double density;          ///< Air density [kg/m³]
        double temperature;      ///< Air temperature [K]
        double pressure;         ///< Air pressure [Pa]
        Vector3D wind_velocity;  ///< Wind velocity [m/s] (East, North, Up)
        
        AtmosphereState(double rho = 1.225, double T = 288.15, double P = 101325.0, 
                       const Vector3D& wind = Vector3D(0, 0, 0))
            : density(rho), temperature(T), pressure(P), wind_velocity(wind) {}
    };
    
    virtual ~AtmosphereModel() = default;
    
    /**
     * @brief Calculate atmospheric properties at given position and time
     * @param position Position in Earth-fixed frame [m]
     * @param time Simulation time [s]
     * @return Atmospheric state
     */
    virtual AtmosphereState calculate_atmosphere(const Vector3D& position, double time = 0.0) const = 0;
    
    /**
     * @brief Get atmosphere model level (1-3)
     */
    virtual int get_level() const = 0;
    
    /**
     * @brief Get atmosphere model name
     */
    virtual std::string get_name() const = 0;
};

/**
 * @brief Level 1: Simple constant atmosphere model
 * @details Constant density, temperature, and pressure. No wind effects.
 *          Suitable for basic simulations and quick calculations.
 */
class SimpleAtmosphere : public AtmosphereModel {
private:
    double density_;     ///< Constant air density [kg/m³]
    double temperature_; ///< Constant air temperature [K]
    double pressure_;    ///< Constant air pressure [Pa]
    
public:
    /**
     * @brief Constructor with standard sea-level conditions
     * @param density Air density [kg/m³] (default: 1.225)
     * @param temperature Air temperature [K] (default: 288.15)
     * @param pressure Air pressure [Pa] (default: 101325.0)
     */
    SimpleAtmosphere(double density = 1.225, double temperature = 288.15, double pressure = 101325.0)
        : density_(density), temperature_(temperature), pressure_(pressure) {}
    
    AtmosphereState calculate_atmosphere(const Vector3D& position, double time = 0.0) const override {
        return AtmosphereState(density_, temperature_, pressure_, Vector3D(0, 0, 0));
    }
    
    int get_level() const override { return 1; }
    std::string get_name() const override { return "Simple"; }
    
    // Getters
    double get_density() const { return density_; }
    double get_temperature() const { return temperature_; }
    double get_pressure() const { return pressure_; }
};

/**
 * @brief Level 2: ISA Standard Atmosphere model
 * @details International Standard Atmosphere (ISA) with altitude-dependent properties.
 *          Supports troposphere (0-11km) and lower stratosphere (11-20km).
 *          Includes constant wind capability.
 */
class StandardAtmosphere : public AtmosphereModel {
private:
    /**
     * @brief ISA atmospheric layer definition
     */
    struct ISALayer {
        double altitude_base;    ///< Base altitude [m]
        double temperature_base; ///< Base temperature [K]
        double pressure_base;    ///< Base pressure [Pa]
        double lapse_rate;       ///< Temperature lapse rate [K/m]
        
        ISALayer(double alt, double temp, double press, double lapse)
            : altitude_base(alt), temperature_base(temp), pressure_base(press), lapse_rate(lapse) {}
    };
    
    std::vector<ISALayer> layers_;  ///< ISA atmospheric layers
    Vector3D constant_wind_;        ///< Constant wind velocity [m/s]
    
    // Physical constants
    static constexpr double G = 9.80665;      ///< Standard gravity [m/s²]
    static constexpr double M = 0.0289644;    ///< Molar mass of air [kg/mol]
    static constexpr double R = 8.31447;      ///< Universal gas constant [J/(mol·K)]
    
    /**
     * @brief Initialize ISA standard layers
     */
    void initialize_isa_layers() {
        layers_.clear();
        // Troposphere: 0 - 11,000 m
        layers_.emplace_back(0.0, 288.15, 101325.0, -0.0065);
        // Lower Stratosphere: 11,000 - 20,000 m  
        layers_.emplace_back(11000.0, 216.65, 22632.1, 0.0);
    }
    
    /**
     * @brief Find appropriate ISA layer for given altitude
     * @param altitude Altitude above sea level [m]
     * @return Reference to ISA layer
     */
    const ISALayer& find_layer(double altitude) const {
        for (size_t i = layers_.size(); i > 0; --i) {
            if (altitude >= layers_[i-1].altitude_base) {
                return layers_[i-1];
            }
        }
        return layers_[0]; // Default to first layer
    }
    
public:
    /**
     * @brief Constructor with optional constant wind
     * @param wind Constant wind velocity [m/s] (default: no wind)
     */
    StandardAtmosphere(const Vector3D& wind = Vector3D(0, 0, 0))
        : constant_wind_(wind) {
        initialize_isa_layers();
    }
    
    AtmosphereState calculate_atmosphere(const Vector3D& position, double time = 0.0) const override {
        double altitude = position.z(); // Altitude is z-component
        const ISALayer& layer = find_layer(altitude);
        
        // Temperature calculation
        double temperature = layer.temperature_base + layer.lapse_rate * (altitude - layer.altitude_base);
        
        // Pressure calculation
        double pressure;
        if (std::abs(layer.lapse_rate) < 1e-9) {
            // Isothermal layer (constant temperature)
            pressure = layer.pressure_base * std::exp(-G * M * (altitude - layer.altitude_base) / (R * temperature));
        } else {
            // Linear temperature change
            double temp_ratio = temperature / layer.temperature_base;
            pressure = layer.pressure_base * std::pow(temp_ratio, -G * M / (R * layer.lapse_rate));
        }
        
        // Density calculation (ideal gas law)
        double density = pressure * M / (R * temperature);
        
        return AtmosphereState(density, temperature, pressure, constant_wind_);
    }
    
    int get_level() const override { return 2; }
    std::string get_name() const override { return "Standard"; }
    
    /**
     * @brief Set constant wind velocity
     * @param wind Wind velocity [m/s] (East, North, Up)
     */
    void set_constant_wind(const Vector3D& wind) { constant_wind_ = wind; }
    
    /**
     * @brief Get current constant wind
     */
    Vector3D get_constant_wind() const { return constant_wind_; }
};

/**
 * @brief Level 3: Dynamic atmosphere model with wind and gust effects
 * @details Extends StandardAtmosphere with dynamic wind modeling including:
 *          - Time-varying wind patterns
 *          - Gust effects
 *          - Turbulence modeling
 */
class DynamicAtmosphere : public StandardAtmosphere {
private:
    double gust_intensity_;     ///< Gust intensity [m/s]
    double gust_frequency_;     ///< Gust frequency [Hz]
    Vector3D turbulence_scale_; ///< Turbulence length scales [m]
    
    /**
     * @brief Calculate time-varying wind component
     * @param position Position [m]
     * @param time Time [s]
     * @return Wind velocity [m/s]
     */
    Vector3D calculate_wind(const Vector3D& position, double time) const {
        // Base constant wind
        Vector3D wind = get_constant_wind();
        
        // Add altitude-dependent wind shear (simplified model)
        double altitude = position.z();
        double wind_shear_factor = 1.0 + 0.1 * std::log(1.0 + altitude / 100.0); // Log wind profile
        wind = wind * wind_shear_factor;
        
        return wind;
    }
    
    /**
     * @brief Calculate gust effects
     * @param position Position [m]
     * @param time Time [s]
     * @return Gust velocity component [m/s]
     */
    Vector3D calculate_gust(const Vector3D& position, double time) const {
        if (gust_intensity_ <= 0.0) {
            return Vector3D(0, 0, 0);
        }
        
        // Sinusoidal gust model with spatial variation
        double phase_x = position.x() / turbulence_scale_.x();
        double phase_y = position.y() / turbulence_scale_.y();
        double phase_z = position.z() / turbulence_scale_.z();
        
        double gust_x = gust_intensity_ * std::sin(2.0 * M_PI * gust_frequency_ * time + phase_x);
        double gust_y = gust_intensity_ * std::sin(2.0 * M_PI * gust_frequency_ * time * 1.3 + phase_y);
        double gust_z = gust_intensity_ * 0.5 * std::sin(2.0 * M_PI * gust_frequency_ * time * 0.7 + phase_z);
        
        return Vector3D(gust_x, gust_y, gust_z);
    }
    
public:
    /**
     * @brief Constructor with wind and gust parameters
     * @param wind Base constant wind [m/s]
     * @param gust_intensity Gust intensity [m/s] (default: 5.0)
     * @param gust_frequency Gust frequency [Hz] (default: 0.1)
     */
    DynamicAtmosphere(const Vector3D& wind = Vector3D(0, 0, 0),
                      double gust_intensity = 5.0,
                      double gust_frequency = 0.1)
        : StandardAtmosphere(wind), gust_intensity_(gust_intensity), gust_frequency_(gust_frequency),
          turbulence_scale_(Vector3D(100.0, 100.0, 50.0)) {}
    
    AtmosphereState calculate_atmosphere(const Vector3D& position, double time = 0.0) const override {
        // Get base ISA atmosphere
        AtmosphereState state = StandardAtmosphere::calculate_atmosphere(position, time);
        
        // Add dynamic wind effects
        Vector3D dynamic_wind = calculate_wind(position, time);
        Vector3D gust_wind = calculate_gust(position, time);
        
        state.wind_velocity = dynamic_wind + gust_wind;
        
        return state;
    }
    
    int get_level() const override { return 3; }
    std::string get_name() const override { return "Dynamic"; }
    
    /**
     * @brief Set gust parameters
     * @param intensity Gust intensity [m/s]
     * @param frequency Gust frequency [Hz]
     */
    void set_gust_parameters(double intensity, double frequency) {
        gust_intensity_ = intensity;
        gust_frequency_ = frequency;
    }
    
    /**
     * @brief Set turbulence length scales
     * @param scale Turbulence scales [m] (x, y, z)
     */
    void set_turbulence_scale(const Vector3D& scale) {
        turbulence_scale_ = scale;
    }
    
    // Getters
    double get_gust_intensity() const { return gust_intensity_; }
    double get_gust_frequency() const { return gust_frequency_; }
    Vector3D get_turbulence_scale() const { return turbulence_scale_; }
};

} // namespace ignis

#endif // IGNIS_PHYSICS_ATMOSPHERE_HPP
