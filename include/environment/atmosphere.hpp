#ifndef IGNISYEET_ENVIRONMENT_ATMOSPHERE_HPP
#define IGNISYEET_ENVIRONMENT_ATMOSPHERE_HPP

#include "../physics/vector3d.hpp"
#include "../physics/state.hpp"
#include "../parameter.hpp"

namespace IgnisYeet::Environment {

/**
 * @brief Atmospheric state at a given position
 */
struct AtmosphericState {
    double density;      // kg/m³
    double pressure;     // Pa
    double temperature;  // K
    double sound_speed;  // m/s
    
    AtmosphericState() 
        : density(1.225)
        , pressure(101325.0)
        , temperature(288.15)
        , sound_speed(343.0)
    {}
    
    AtmosphericState(double d, double p, double t, double s)
        : density(d), pressure(p), temperature(t), sound_speed(s)
    {}
};

/**
 * @brief Base class for atmosphere models
 */
class AtmosphereModel {
private:
    // Missing member variables for atmosphere.cpp compatibility
    int physics_level_;
    double sea_level_pressure_;
    double sea_level_temperature_;
    double sea_level_density_;
    double temperature_lapse_rate_;
    double gas_constant_;
    double gamma_;
    double scale_height_;
    enum class AtmosphereModelType { EXPONENTIAL, ISA };
    AtmosphereModelType model_type_;
    
public:
    virtual ~AtmosphereModel() = default;
    
    // Missing methods for atmosphere.cpp compatibility
    void initialize_from_parameters(const Parameter& params);
    bool initialize();
    void reset();
    AtmosphericState get_atmospheric_state(double altitude) const;
    AtmosphericState compute_exponential_atmosphere(double altitude) const;
    AtmosphericState compute_isa_atmosphere(double altitude) const;
    
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
    Environment(const Parameter& params);  // Add Parameter constructor
    ~Environment() = default;
    
    // Non-copyable but movable
    Environment(const Environment&) = delete;
    Environment& operator=(const Environment&) = delete;
    Environment(Environment&&) = default;
    Environment& operator=(Environment&&) = default;
    
    // Initialization and reset
    bool initialize();
    void reset();
    void initialize_from_parameters(const Parameter& params);
    
    // Get complete environment state
    Physics::EnvironmentState state_at(
        const Physics::Vector3D& position, double time) const;
    Physics::EnvironmentState get_state(
        const Physics::Vector3D& position, double time) const;  // Alternative interface
        
    // Gravity force computation
    Physics::Vector3D compute_gravity_force(
        const Physics::Vector3D& position, double mass) const;
    
    // Individual components
    const AtmosphereModel& atmosphere() const { return *atmosphere_; }
    const WindModel& wind() const { return *wind_; }
    const GravityModel& gravity() const { return *gravity_; }
};

/**
 * @brief Standard atmosphere model implementation 
 */
class StandardAtmosphereModel : public AtmosphereModel {
private:
    double sea_level_density_;
    double sea_level_pressure_;
    double sea_level_temperature_;
    double scale_height_;
    
public:
    StandardAtmosphereModel();
    StandardAtmosphereModel(const AtmosphereConfig& config);
    ~StandardAtmosphereModel() = default;
    
    // Implement pure virtual methods
    double density(double altitude) const override;
    double pressure(double altitude) const override;
    double temperature(double altitude) const override;
    double sound_speed(double altitude) const override;
    Physics::EnvironmentState environment_at(const Physics::Vector3D& position) const override;
};

} // namespace IgnisYeet::Environment

#endif // IGNISYEET_ENVIRONMENT_ATMOSPHERE_HPP
