#ifndef IGNISYEET_ROCKET_VEHICLE_HPP
#define IGNISYEET_ROCKET_VEHICLE_HPP

#include "../physics/state.hpp"
#include "../physics/vector3d.hpp"
#include "../physics/quaternion.hpp"
#include "../physics/aerodynamics.hpp"
#include "../parameter.hpp"
#include <memory>
#include <vector>

namespace IgnisYeet::Rocket {

/**
 * @brief Vehicle configuration structure
 */
struct VehicleConfig {
    // Mass properties
    double mass_dry;                     // Dry mass [kg]
    double mass_propellant;              // Propellant mass [kg]
    
    // Geometry
    double length;                       // Total length [m]
    double diameter;                     // Diameter [m]
    double reference_area;               // Reference area [m²]
    
    // Aerodynamics
    double normal_force_coefficient;     // Normal force coefficient
    double drag_coefficient;             // Drag coefficient
    double lift_coefficient;             // Lift coefficient
    
    // Inertia
    double inertia_xx;                   // Moment of inertia around x-axis [kg·m²]
    double inertia_yy;                   // Moment of inertia around y-axis [kg·m²]
    double inertia_zz;                   // Moment of inertia around z-axis [kg·m²]
    
    VehicleConfig() 
        : mass_dry(5.0)
        , mass_propellant(2.0)
        , length(1.0)
        , diameter(0.1)
        , reference_area(0.00785)
        , normal_force_coefficient(0.1)
        , drag_coefficient(0.5)
        , lift_coefficient(0.2)
        , inertia_xx(0.1)
        , inertia_yy(0.1)
        , inertia_zz(0.01)
    {}
};

/**
 * @brief Propulsion configuration structure  
 */
struct PropulsionConfig {
    std::string engine_type;
    std::vector<std::pair<double, double>> thrust_curve;  // time-thrust pairs
    double isp;            // Specific impulse [s]
    double burn_time;
    double thrust;         // Maximum thrust [N]
    double specific_impulse; // Specific impulse [s] (alias for isp)
    double propellant_mass;  // Propellant mass [kg]
    
    PropulsionConfig() 
        : engine_type("solid")
        , isp(200.0)
        , burn_time(5.0)
        , thrust(100.0)
        , specific_impulse(200.0)
        , propellant_mass(2.0)
    {}
};

/**
 * @brief Rocket mass properties
 */
struct MassProperties {
    double dry_mass;           // Dry mass [kg]
    double propellant_mass;    // Current propellant mass [kg]
    double total_mass() const { return dry_mass + propellant_mass; }
    
    // Center of mass and moments of inertia
    Physics::Vector3D center_of_mass;  // Center of mass [m]
    Physics::Vector3D moments_of_inertia; // [Ixx, Iyy, Izz] [kg⋅m²]
};

/**
 * @brief Rocket geometry
 */
struct Geometry {
    double length;             // Total length [m]
    double diameter;           // Maximum diameter [m]
    double nose_cone_length;   // Nose cone length [m]
    double body_tube_length;   // Body tube length [m]
    double fin_span;           // Fin span [m]
    double reference_area;     // Reference area for aerodynamics [m²]
    
    // Aerodynamic reference point
    Physics::Vector3D aerodynamic_center; // Aerodynamic center [m]
    Physics::Vector3D center_of_pressure;  // Center of pressure [m]
};

/**
 * @brief Propulsion system
 */
class PropulsionSystem {
private:
    double burn_time_;
    double total_impulse_;
    double mass_flow_rate_;
    std::vector<std::pair<double, double>> thrust_curve_; // (time, thrust)
    
    // Missing member variables
    double max_thrust_;
    double specific_impulse_;
    bool is_burning_;
    double total_impulse_delivered_;
    Physics::Vector3D thrust_direction_;
    
public:
    PropulsionSystem(const PropulsionConfig& config);
    
    // Configuration
    void configure(const PropulsionConfig& config);
    void initialize_from_config(const VehicleConfig& config);
    
    // Missing methods
    void initialize_from_parameters(const Parameter& params);
    bool initialize();
    void reset();
    double get_thrust(double time) const;
    
    // Get thrust at given time
    double thrust(double time) const;
    
    // Get mass flow rate at given time
    double mass_flow_rate(double time) const;
    
    // Check if motor is burning
    bool is_burning(double time) const;
    
    // Get total burn time
    double burn_time() const { return burn_time_; }
    
    // Get total impulse
    double total_impulse() const { return total_impulse_; }
};

/**
 * @brief Recovery system
 */
class RecoverySystem {
private:
    double deployment_altitude_;
    double drogue_area_;
    double main_area_;
    double drogue_cd_;
    double main_cd_;
    bool drogue_deployed_;
    bool main_deployed_;
    
public:
    RecoverySystem(const RecoveryConfig& config);
    
    // Configuration
    void initialize_from_config(const VehicleConfig& config);
    
    // Update deployment status based on flight conditions
    void update(const Physics::RigidBodyState& state);
    
    // Get drag coefficient and area
    double drag_coefficient() const;
    double drag_area() const;
    
    // Deployment status
    bool is_drogue_deployed() const { return drogue_deployed_; }
    bool is_main_deployed() const { return main_deployed_; }
};

/**
 * @brief Complete rocket vehicle
 */
class Vehicle {
private:
    // Components
    std::unique_ptr<PropulsionSystem> propulsion_;
    std::unique_ptr<RecoverySystem> recovery_;
    std::unique_ptr<ignis::physics::AerodynamicsModel> aerodynamics_;
    
    // Properties
    MassProperties mass_props_;
    Geometry geometry_;
    double current_mass_;  // Current total mass [kg]
    
    // Launch conditions
    Physics::Vector3D launch_position_;
    Physics::Quaternion launch_orientation_;
    
    // Missing member variables for vehicle.cpp compatibility
    VehicleConfig config_;
    
public:
    Vehicle(const RocketConfig& config);
    Vehicle(const VehicleConfig& config);  // Add VehicleConfig constructor
    Vehicle(const Parameter& params);  // Add Parameter constructor
    ~Vehicle() = default;
    
    // Non-copyable but movable
    Vehicle(const Vehicle&) = delete;
    Vehicle& operator=(const Vehicle&) = delete;
    Vehicle(Vehicle&&) = default;
    Vehicle& operator=(Vehicle&&) = default;
    
    // Initialization and reset
    bool initialize();
    void reset();
    void initialize_from_parameters(const Parameter& params);
    
    // State initialization
    Physics::RigidBodyState initial_state() const;
    
    // Update mass properties during flight
    void update_mass(double time);
    
    // Get current mass
    double mass(double time) const;
    double get_mass(double time) const;  // Alternative interface
    
    // Propulsion
    double get_thrust(double time) const;
    double get_mass_flow_rate(double time) const;
    double thrust(double time) const;
    Physics::Vector3D thrust_vector(double time, const Physics::Quaternion& orientation) const;
    
    // Aerodynamics
    Physics::ForcesMoments aerodynamic_forces(
        const Physics::RigidBodyState& state,
        const Physics::EnvironmentState& env) const;
    
    // Get aerodynamics model
    const ignis::physics::AerodynamicsModel& aerodynamics() const { return *aerodynamics_; }
    
    // Recovery
    void update_recovery(const Physics::RigidBodyState& state);
    Physics::Vector3D recovery_drag(
        const Physics::RigidBodyState& state,
        const Physics::EnvironmentState& env) const;
        
    // Aerodynamics
    Physics::ForcesMoments compute_aerodynamics(
        const Physics::RigidBodyState& state,
        const Physics::EnvironmentState& env) const;
    
    // Accessors
    const MassProperties& mass_properties() const { return mass_props_; }
    const Geometry& geometry() const { return geometry_; }
    const PropulsionSystem& propulsion() const { return *propulsion_; }
    const RecoverySystem& recovery() const { return *recovery_; }
    
    // Launch conditions
    const Physics::Vector3D& launch_position() const { return launch_position_; }
    const Physics::Quaternion& launch_orientation() const { return launch_orientation_; }
    
    // Component access
    PropulsionSystem& get_propulsion_system() { return propulsion_system_; }
    const PropulsionSystem& get_propulsion_system() const { return propulsion_system_; }
    
    // Flight phase detection
    enum class FlightPhase {
        PRE_LAUNCH,
        POWERED_FLIGHT,
        COASTING,
        RECOVERY,
        LANDED
    };
    
    FlightPhase current_phase(double time, const Physics::RigidBodyState& state) const;
};

} // namespace IgnisYeet::Rocket

#endif // IGNISYEET_ROCKET_VEHICLE_HPP
