#ifndef IGNISYEET_ENVIRONMENT_ATMOSPHERE_HPP
#define IGNISYEET_ENVIRONMENT_ATMOSPHERE_HPP

#include "../physics/vector3d.hpp"
#include "../physics/state.hpp"
#include "../parameter.hpp"

namespace IgnisYeet::Environment {

/**
 * @brief Base class for atmosphere models
 */
class AtmosphereModel {
public:
    virtual ~AtmosphereModel() = default;
    
    // Get atmospheric properties at given altitude
    virtual double density(double altitude) const = 0;
    virtual double pressure(double altitude) const = 0;
    virtual double temperature(double altitude) const = 0;
    virtual double sound_speed(double altitude) const = 0;
    
    // Get complete environment state
    virtual Physics::EnvironmentState environment_at(
        const Physics::Vector3D& position) const = 0;
};

/**
 * @brief Simple exponential atmosphere model
 */
class ExponentialAtmosphere : public AtmosphereModel {
private:
    double sea_level_density_;
    double sea_level_pressure_;
    double sea_level_temperature_;
    double scale_height_;
    
public:
    ExponentialAtmosphere();
    ExponentialAtmosphere(double rho0, double p0, double T0, double H);
    
    double density(double altitude) const override;
    double pressure(double altitude) const override;
    double temperature(double altitude) const override;
    double sound_speed(double altitude) const override;
    
    Physics::EnvironmentState environment_at(
        const Physics::Vector3D& position) const override;
};

/**
 * @brief US Standard Atmosphere 1976 model
 */
class StandardAtmosphere : public AtmosphereModel {
private:
    struct Layer {
        double altitude_base;
        double temperature_base;
        double pressure_base;
        double density_base;
        double temperature_gradient;
    };
    
    static constexpr int NUM_LAYERS = 7;
    static const Layer layers_[NUM_LAYERS];
    
    const Layer& get_layer(double altitude) const;
    
public:
    StandardAtmosphere() = default;
    
    double density(double altitude) const override;
    double pressure(double altitude) const override;
    double temperature(double altitude) const override;
    double sound_speed(double altitude) const override;
    
    Physics::EnvironmentState environment_at(
        const Physics::Vector3D& position) const override;
};

/**
 * @brief Wind model
 */
class WindModel {
public:
    virtual ~WindModel() = default;
    
    // Get wind velocity at given position and time
    virtual Physics::Vector3D wind_at(
        const Physics::Vector3D& position, double time) const = 0;
};

/**
 * @brief Constant wind model
 */
class ConstantWind : public WindModel {
private:
    Physics::Vector3D wind_velocity_;
    
public:
    ConstantWind() : wind_velocity_(Physics::Vector3D::zero()) {}
    ConstantWind(const Physics::Vector3D& wind) : wind_velocity_(wind) {}
    
    Physics::Vector3D wind_at(
        const Physics::Vector3D& position, double time) const override {
        return wind_velocity_;
    }
};

/**
 * @brief Altitude-dependent wind model
 */
class AltitudeWind : public WindModel {
private:
    Physics::Vector3D surface_wind_;
    double wind_scale_height_;
    
public:
    AltitudeWind(const Physics::Vector3D& surface_wind, double scale_height);
    
    Physics::Vector3D wind_at(
        const Physics::Vector3D& position, double time) const override;
};

/**
 * @brief Gravity model
 */
class GravityModel {
public:
    virtual ~GravityModel() = default;
    
    // Get gravitational acceleration at given position
    virtual Physics::Vector3D gravity_at(
        const Physics::Vector3D& position) const = 0;
};

/**
 * @brief Constant gravity model
 */
class ConstantGravity : public GravityModel {
private:
    Physics::Vector3D gravity_;
    
public:
    ConstantGravity() : gravity_(Physics::Vector3D(0.0, 0.0, -9.80665)) {}
    ConstantGravity(const Physics::Vector3D& g) : gravity_(g) {}
    
    Physics::Vector3D gravity_at(
        const Physics::Vector3D& position) const override {
        return gravity_;
    }
};

/**
 * @brief Altitude-dependent gravity model
 */
class AltitudeGravity : public GravityModel {
private:
    double surface_gravity_;
    double earth_radius_;
    
public:
    AltitudeGravity();
    AltitudeGravity(double g0, double R);
    
    Physics::Vector3D gravity_at(
        const Physics::Vector3D& position) const override;
};

/**
 * @brief Complete environment system
 */
class Environment {
private:
    std::unique_ptr<AtmosphereModel> atmosphere_;
    std::unique_ptr<WindModel> wind_;
    std::unique_ptr<GravityModel> gravity_;
    
public:
    Environment(const EnvironmentConfig& config);
    ~Environment() = default;
    
    // Non-copyable but movable
    Environment(const Environment&) = delete;
    Environment& operator=(const Environment&) = delete;
    Environment(Environment&&) = default;
    Environment& operator=(Environment&&) = default;
    
    // Get complete environment state
    Physics::EnvironmentState state_at(
        const Physics::Vector3D& position, double time) const;
    
    // Individual components
    const AtmosphereModel& atmosphere() const { return *atmosphere_; }
    const WindModel& wind() const { return *wind_; }
    const GravityModel& gravity() const { return *gravity_; }
};

} // namespace IgnisYeet::Environment

#endif // IGNISYEET_ENVIRONMENT_ATMOSPHERE_HPP
