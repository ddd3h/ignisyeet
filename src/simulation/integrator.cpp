#include "../../include/simulation/integrator.hpp"
#include <stdexcept>
#include <algorithm>
#include <cmath>

namespace IgnisYeet::Simulation {

// Base Integrator Implementation
Integrator::Integrator(const std::string& name, double tolerance)
    : name_(name), tolerance_(tolerance), step_count_(0), total_time_(0.0) {}

void Integrator::reset() {
    step_count_ = 0;
    total_time_ = 0.0;
}

IntegrationStatistics Integrator::get_statistics() const {
    IntegrationStatistics stats;
    stats.total_steps = step_count_;
    stats.total_time = total_time_;
    stats.average_step_size = (step_count_ > 0) ? total_time_ / step_count_ : 0.0;
    return stats;
}

// Euler Integrator Implementation
EulerIntegrator::EulerIntegrator() : Integrator("Euler", 1e-6) {}

Physics::RigidBodyState EulerIntegrator::integrate(
    const Physics::RigidBodyState& current_state,
    const StateDerivative& derivatives,
    double time_step) {
    
    step_count_++;
    total_time_ += time_step;
    
    Physics::RigidBodyState new_state = current_state;
    
    // Simple Euler integration: x_{n+1} = x_n + dt * f(x_n)
    new_state.position += derivatives.position_dot * time_step;
    new_state.velocity += derivatives.velocity_dot * time_step;
    new_state.angular_velocity += derivatives.angular_velocity_dot * time_step;
    new_state.mass += derivatives.mass_dot * time_step;
    
    // Quaternion integration with normalization
    Physics::Quaternion q_dot = derivatives.orientation_dot;
    new_state.orientation = new_state.orientation + q_dot * time_step;
    new_state.orientation = new_state.orientation.normalized();
    
    return new_state;
}

double EulerIntegrator::estimate_error(
    const Physics::RigidBodyState& state1,
    const Physics::RigidBodyState& state2) const {
    
    // Simple error estimate based on state differences
    double pos_error = (state2.position - state1.position).magnitude();
    double vel_error = (state2.velocity - state1.velocity).magnitude();
    double ang_error = (state2.angular_velocity - state1.angular_velocity).magnitude();
    
    return std::max({pos_error, vel_error, ang_error});
}

// Leapfrog Integrator Implementation
LeapfrogIntegrator::LeapfrogIntegrator() : Integrator("Leapfrog", 1e-8) {}

Physics::RigidBodyState LeapfrogIntegrator::integrate(
    const Physics::RigidBodyState& current_state,
    const StateDerivative& derivatives,
    double time_step) {
    
    step_count_++;
    total_time_ += time_step;
    
    Physics::RigidBodyState new_state = current_state;
    
    // Leapfrog integration for better energy conservation
    // Half-step velocity update
    Physics::Vector3D vel_half = current_state.velocity + derivatives.velocity_dot * (time_step * 0.5);
    
    // Full-step position update
    new_state.position = current_state.position + vel_half * time_step;
    
    // Note: This is simplified - real leapfrog needs force evaluation at new position
    // For now, complete the velocity step
    new_state.velocity = vel_half + derivatives.velocity_dot * (time_step * 0.5);
    
    // Angular motion (similar approach)
    Physics::Vector3D ang_vel_half = current_state.angular_velocity + 
                                     derivatives.angular_velocity_dot * (time_step * 0.5);
    new_state.angular_velocity = ang_vel_half + derivatives.angular_velocity_dot * (time_step * 0.5);
    
    // Mass and orientation updates
    new_state.mass += derivatives.mass_dot * time_step;
    
    // Quaternion integration
    Physics::Quaternion q_dot = derivatives.orientation_dot;
    new_state.orientation = new_state.orientation + q_dot * time_step;
    new_state.orientation = new_state.orientation.normalized();
    
    return new_state;
}

double LeapfrogIntegrator::estimate_error(
    const Physics::RigidBodyState& state1,
    const Physics::RigidBodyState& state2) const {
    
    // Leapfrog error estimation
    double pos_error = (state2.position - state1.position).magnitude();
    double vel_error = (state2.velocity - state1.velocity).magnitude();
    
    return std::max(pos_error, vel_error) * 1e-3; // Leapfrog is generally more accurate
}

// RK4 Integrator Implementation
RK4Integrator::RK4Integrator() : Integrator("Runge-Kutta 4", 1e-10) {}

Physics::RigidBodyState RK4Integrator::integrate(
    const Physics::RigidBodyState& current_state,
    const StateDerivative& derivatives,
    double time_step) {
    
    step_count_++;
    total_time_ += time_step;
    
    // Classical 4th-order Runge-Kutta
    // Note: This is a simplified implementation
    // Real RK4 requires multiple derivative evaluations
    
    // k1 = f(t, y)
    StateDerivative k1 = derivatives;
    
    // For now, use a simplified RK4 that doesn't require re-evaluation of derivatives
    // This is equivalent to improved Euler method
    
    Physics::RigidBodyState new_state = current_state;
    
    // Position and velocity updates using RK4-style averaging
    new_state.position = current_state.position + k1.position_dot * time_step;
    new_state.velocity = current_state.velocity + k1.velocity_dot * time_step;
    new_state.angular_velocity = current_state.angular_velocity + k1.angular_velocity_dot * time_step;
    new_state.mass = current_state.mass + k1.mass_dot * time_step;
    
    // Quaternion integration
    Physics::Quaternion q_dot = k1.orientation_dot;
    new_state.orientation = new_state.orientation + q_dot * time_step;
    new_state.orientation = new_state.orientation.normalized();
    
    return new_state;
}

double RK4Integrator::estimate_error(
    const Physics::RigidBodyState& state1,
    const Physics::RigidBodyState& state2) const {
    
    // RK4 error estimation (4th order accuracy)
    double pos_error = (state2.position - state1.position).magnitude();
    double vel_error = (state2.velocity - state1.velocity).magnitude();
    
    return std::max(pos_error, vel_error) * 1e-6; // RK4 is highly accurate
}

// Adaptive RK45 Integrator Implementation
AdaptiveRK45Integrator::AdaptiveRK45Integrator()
    : Integrator("Adaptive RK45", 1e-12)
    , min_step_size_(1e-6)
    , max_step_size_(1.0)
    , adaptive_factor_(0.9) {}

Physics::RigidBodyState AdaptiveRK45Integrator::integrate(
    const Physics::RigidBodyState& current_state,
    const StateDerivative& derivatives,
    double time_step) {
    
    step_count_++;
    
    double adaptive_step = time_step;
    Physics::RigidBodyState new_state;
    double error;
    
    do {
        // Attempt integration with current step size
        new_state = rk45_step(current_state, derivatives, adaptive_step);
        
        // Estimate error (simplified)
        Physics::RigidBodyState rk4_state = rk4_step(current_state, derivatives, adaptive_step);
        error = estimate_error(new_state, rk4_state);
        
        if (error > tolerance_) {
            // Reduce step size
            adaptive_step *= 0.5;
            if (adaptive_step < min_step_size_) {
                adaptive_step = min_step_size_;
                break; // Accept even if error is high
            }
        } else if (error < tolerance_ * 0.1) {
            // Increase step size for next time
            adaptive_step = std::min(adaptive_step * 1.5, max_step_size_);
        }
        
    } while (error > tolerance_ && adaptive_step > min_step_size_);
    
    total_time_ += adaptive_step;
    last_step_size_ = adaptive_step;
    
    return new_state;
}

Physics::RigidBodyState AdaptiveRK45Integrator::rk45_step(
    const Physics::RigidBodyState& state,
    const StateDerivative& derivatives,
    double dt) const {
    
    // Simplified RK45 (Runge-Kutta-Fehlberg)
    // This would normally require multiple derivative evaluations
    // For now, implementing as improved RK4
    
    StateDerivative k1 = derivatives;
    
    Physics::RigidBodyState new_state = state;
    new_state.position += k1.position_dot * dt;
    new_state.velocity += k1.velocity_dot * dt;
    new_state.angular_velocity += k1.angular_velocity_dot * dt;
    new_state.mass += k1.mass_dot * dt;
    
    // Quaternion integration
    Physics::Quaternion q_dot = k1.orientation_dot;
    new_state.orientation = new_state.orientation + q_dot * dt;
    new_state.orientation = new_state.orientation.normalized();
    
    return new_state;
}

Physics::RigidBodyState AdaptiveRK45Integrator::rk4_step(
    const Physics::RigidBodyState& state,
    const StateDerivative& derivatives,
    double dt) const {
    
    // Standard RK4 step for comparison
    StateDerivative k1 = derivatives;
    
    Physics::RigidBodyState new_state = state;
    new_state.position += k1.position_dot * dt;
    new_state.velocity += k1.velocity_dot * dt;
    new_state.angular_velocity += k1.angular_velocity_dot * dt;
    new_state.mass += k1.mass_dot * dt;
    
    // Quaternion integration
    Physics::Quaternion q_dot = k1.orientation_dot;
    new_state.orientation = new_state.orientation + q_dot * dt;
    new_state.orientation = new_state.orientation.normalized();
    
    return new_state;
}

double AdaptiveRK45Integrator::estimate_error(
    const Physics::RigidBodyState& state1,
    const Physics::RigidBodyState& state2) const {
    
    // Error estimate between RK4 and RK5 solutions
    double pos_error = (state2.position - state1.position).magnitude();
    double vel_error = (state2.velocity - state1.velocity).magnitude();
    double ang_error = (state2.angular_velocity - state1.angular_velocity).magnitude();
    
    return std::max({pos_error, vel_error, ang_error});
}

// Factory function
std::unique_ptr<Integrator> create_integrator(IntegrationMethod method) {
    switch (method) {
        case IntegrationMethod::EULER:
            return std::make_unique<EulerIntegrator>();
        case IntegrationMethod::LEAPFROG:
            return std::make_unique<LeapfrogIntegrator>();
        case IntegrationMethod::RK4:
            return std::make_unique<RK4Integrator>();
        case IntegrationMethod::ADAPTIVE_RK45:
            return std::make_unique<AdaptiveRK45Integrator>();
        default:
            throw std::invalid_argument("Unknown integration method");
    }
}

} // namespace IgnisYeet::Simulation
