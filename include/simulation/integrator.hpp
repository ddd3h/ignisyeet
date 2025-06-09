#ifndef IGNISYEET_SIMULATION_INTEGRATOR_HPP
#define IGNISYEET_SIMULATION_INTEGRATOR_HPP

#include "../physics/state.hpp"
#include "../physics/vector3d.hpp"
#include "../physics/quaternion.hpp"
#include <functional>
#include <memory>
#include <string>
#include <cmath>
#include <algorithm>

namespace IgnisYeet::Simulation {

// Forward declaration for parameter types
enum class IntegrationMethod {
    EULER,
    LEAPFROG,
    RK4,
    RK45,
    ADAPTIVE_RK45
};

namespace IgnisYeet::Simulation {

/**
 * @brief State derivative for integration
 */
struct StateDerivative {
    Physics::Vector3D position_dot;      // Velocity
    Physics::Vector3D velocity_dot;      // Acceleration
    Physics::Quaternion orientation_dot; // Quaternion derivative
    Physics::Vector3D angular_velocity_dot; // Angular acceleration
    double mass_dot;                     // Mass flow rate
    
    StateDerivative() 
        : position_dot(Physics::Vector3D::zero())
        , velocity_dot(Physics::Vector3D::zero())
        , orientation_dot(Physics::Quaternion::identity())
        , angular_velocity_dot(Physics::Vector3D::zero())
        , mass_dot(0.0)
    {}
};

/**
 * @brief Function type for computing state derivatives
 */
using DerivativeFunction = std::function<StateDerivative(
    const Physics::RigidBodyState&, double)>;

/**
 * @brief Base class for numerical integrators
 */
class Integrator {
public:
    virtual ~Integrator() = default;
    
    // Integrate one time step
    virtual Physics::RigidBodyState step(
        const Physics::RigidBodyState& current_state,
        double dt,
        const DerivativeFunction& derivative_func) = 0;
    
    // Get integrator name
    virtual std::string name() const = 0;
    
    // Get integration order
    virtual int order() const = 0;
};

/**
 * @brief Euler (first-order) integrator
 */
class EulerIntegrator : public Integrator {
public:
    Physics::RigidBodyState step(
        const Physics::RigidBodyState& current_state,
        double dt,
        const DerivativeFunction& derivative_func) override;
    
    std::string name() const override { return "Euler"; }
    int order() const override { return 1; }
};

/**
 * @brief Leapfrog integrator
 */
class LeapfrogIntegrator : public Integrator {
private:
    bool first_step_;
    Physics::Vector3D prev_acceleration_;
    
public:
    LeapfrogIntegrator() : first_step_(true), prev_acceleration_(Physics::Vector3D::zero()) {}
    
    Physics::RigidBodyState step(
        const Physics::RigidBodyState& current_state,
        double dt,
        const DerivativeFunction& derivative_func) override;
    
    std::string name() const override { return "Leapfrog"; }
    int order() const override { return 2; }
    
    void reset() { first_step_ = true; }
};

/**
 * @brief Runge-Kutta 4th order integrator
 */
class RK4Integrator : public Integrator {
public:
    Physics::RigidBodyState step(
        const Physics::RigidBodyState& current_state,
        double dt,
        const DerivativeFunction& derivative_func) override;
    
    std::string name() const override { return "RK4"; }
    int order() const override { return 4; }

private:
    // Helper function to add state and derivative
    Physics::RigidBodyState add_scaled_derivative(
        const Physics::RigidBodyState& state,
        const StateDerivative& derivative,
        double scale) const;
};

/**
 * @brief Adaptive Runge-Kutta integrator with error control
 */
class AdaptiveRK45Integrator : public Integrator {
private:
    double tolerance_;
    double min_dt_;
    double max_dt_;
    
    struct RKResult {
        Physics::RigidBodyState state;
        double error;
        double recommended_dt;
    };
    
    RKResult rk45_step(
        const Physics::RigidBodyState& current_state,
        double dt,
        const DerivativeFunction& derivative_func) const;
    
public:
    AdaptiveRK45Integrator(double tolerance = 1e-6, 
                          double min_dt = 1e-6, 
                          double max_dt = 1.0);
    
    Physics::RigidBodyState step(
        const Physics::RigidBodyState& current_state,
        double dt,
        const DerivativeFunction& derivative_func) override;
    
    std::string name() const override { return "Adaptive RK45"; }
    int order() const override { return 5; }
    
    // Set tolerance
    void set_tolerance(double tol) { tolerance_ = tol; }
    double get_tolerance() const { return tolerance_; }
};

/**
 * @brief Factory for creating integrators
 */
class IntegratorFactory {
public:
    static std::unique_ptr<Integrator> create(IntegrationMethod method);
};

/**
 * @brief Helper functions for state manipulation
 */
namespace StateUtils {
    // Apply derivative to state
    Physics::RigidBodyState apply_derivative(
        const Physics::RigidBodyState& state,
        const StateDerivative& derivative,
        double dt);
    
    // Compute quaternion derivative from angular velocity
    Physics::Quaternion quaternion_derivative(
        const Physics::Quaternion& orientation,
        const Physics::Vector3D& angular_velocity);
    
    // Normalize quaternion in state
    void normalize_orientation(Physics::RigidBodyState& state);
    
    // Compute relative error between two states
    double relative_error(
        const Physics::RigidBodyState& state1,
        const Physics::RigidBodyState& state2);
}

} // namespace IgnisYeet::Simulation

#endif // IGNISYEET_SIMULATION_INTEGRATOR_HPP
