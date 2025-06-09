#include <iostream>
#include <cassert>
#include <cmath>
#include "physics/gravity.hpp"

using namespace ignis;

int main() {
    std::cout << "=== Simple Gravity Integration Demo ===" << std::endl;
    
    // Real rocket scenario
    double altitude = 1000.0;    // 1km altitude [m]
    double velocity = 100.0;     // 100 m/s eastward [m/s]
    
    Vector3D position(0, 0, altitude);
    Vector3D vel_stationary(0, 0, 0);
    Vector3D vel_moving(velocity, 0, 0);
    
    // Create all three gravity models
    UniformGravity level1(9.80665);
    AltitudeDependentGravity level2(9.80665, 6371000.0);
    RotatingEarthGravity level3(9.80665, 6371000.0, 7.2921159e-5);
    
    // Calculate gravity for stationary rocket
    Vector3D g1_stat = level1.calculate_gravity(position, vel_stationary);
    Vector3D g2_stat = level2.calculate_gravity(position, vel_stationary);
    Vector3D g3_stat = level3.calculate_gravity(position, vel_stationary);
    
    std::cout << "\nGravity comparison at " << altitude << "m altitude (stationary):" << std::endl;
    std::cout << "Level 1 (Uniform):    g = " << -g1_stat.z() << " m/s²" << std::endl;
    std::cout << "Level 2 (Altitude):   g = " << -g2_stat.z() << " m/s²" << std::endl;
    std::cout << "Level 3 (Rotating):   g = " << -g3_stat.z() << " m/s²" << std::endl;
    
    double diff_2_1 = -g2_stat.z() - (-g1_stat.z());
    std::cout << "Difference (L2-L1): " << diff_2_1 << " m/s² (" << 
                 (diff_2_1/-g1_stat.z())*100 << "%)" << std::endl;
    
    // Calculate gravity for moving rocket
    Vector3D g1_move = level1.calculate_gravity(position, vel_moving);
    Vector3D g2_move = level2.calculate_gravity(position, vel_moving);
    Vector3D g3_move = level3.calculate_gravity(position, vel_moving);
    
    std::cout << "\nGravity comparison at " << altitude << "m altitude (moving " << velocity << " m/s east):" << std::endl;
    std::cout << "Level 1 (Uniform):    g = " << -g1_move.z() << " m/s²" << std::endl;
    std::cout << "Level 2 (Altitude):   g = " << -g2_move.z() << " m/s²" << std::endl;
    std::cout << "Level 3 (Rotating):   g = " << -g3_move.z() << " m/s²" << std::endl;
    
    double coriolis_effect = -g3_move.z() - (-g2_move.z());
    std::cout << "Coriolis effect: " << coriolis_effect << " m/s²" << std::endl;
    
    // Numerical integration demonstration
    std::cout << "\n--- Numerical Integration Demonstration ---" << std::endl;
    double dt = 0.1;   // Time step [s]
    double total_time = 10.0;  // Total simulation time [s]
    
    // Initial conditions
    Vector3D pos_sim = position;
    Vector3D vel_sim = vel_moving;
    double mass = 100.0;  // kg
    
    std::cout << "Time\tAltitude\tVelocity\tAcceleration" << std::endl;
    std::cout << "[s]\t[m]\t\t[m/s]\t\t[m/s²]" << std::endl;
    
    for (double t = 0; t <= total_time; t += dt) {
        Vector3D gravity_acc = level3.calculate_gravity(pos_sim, vel_sim);
        
        if (int(t * 10) % 10 == 0) {  // Print every 1 second
            std::cout << t << "\t" << pos_sim.z() << "\t\t" << 
                         vel_sim.magnitude() << "\t\t" << gravity_acc.magnitude() << std::endl;
        }
        
        // Simple Euler integration
        vel_sim = vel_sim + gravity_acc * dt;
        pos_sim = pos_sim + vel_sim * dt;
        
        // Stop if hit ground
        if (pos_sim.z() <= 0) {
            std::cout << "Ground impact at t = " << t << " s" << std::endl;
            break;
        }
    }
    
    std::cout << "\n=== Gravity integration demo completed! ===" << std::endl;
    return 0;
}
