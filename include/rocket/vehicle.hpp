#ifndef IGNISYEET_ROCKET_VEHICLE_HPP
#define IGNISYEET_ROCKET_VEHICLE_HPP

#include "../physics/state.hpp"
#include "../physics/vector3d.hpp"
#include "../physics/quaternion.hpp"
#include "../parameter.hpp"
#include <memory>
#include <vector>

namespace IgnisYeet::Rocket {

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
    
public:
    PropulsionSystem(const PropulsionConfig& config);
    
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
    
    // Properties
    MassProperties mass_props_;
    Geometry geometry_;
    
    // Aerodynamic coefficients
    double cd_power_off_;      // Drag coefficient (power off)
    double cd_power_on_;       // Drag coefficient (power on)
    double cl_alpha_;          // Lift curve slope
    double cn_alpha_;          // Normal force curve slope
    double cm_alpha_;          // Pitching moment curve slope
    
    // Launch conditions
    Physics::Vector3D launch_position_;
    Physics::Quaternion launch_orientation_;
    
public:
    Vehicle(const RocketConfig& config);
    ~Vehicle() = default;
    
    // Non-copyable but movable
    Vehicle(const Vehicle&) = delete;
    Vehicle& operator=(const Vehicle&) = delete;
    Vehicle(Vehicle&&) = default;
    Vehicle& operator=(Vehicle&&) = default;
    
    // State initialization
    Physics::RigidBodyState initial_state() const;
    
    // Update mass properties during flight
    void update_mass(double time);
    
    // Get current mass
    double mass(double time) const;
    
    // Propulsion
    double thrust(double time) const;
    Physics::Vector3D thrust_vector(double time, const Physics::Quaternion& orientation) const;
    
    // Aerodynamics
    Physics::ForcesMoments aerodynamic_forces(
        const Physics::RigidBodyState& state,
        const Physics::EnvironmentState& env) const;
    
    // Recovery
    void update_recovery(const Physics::RigidBodyState& state);
    Physics::Vector3D recovery_drag(
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
