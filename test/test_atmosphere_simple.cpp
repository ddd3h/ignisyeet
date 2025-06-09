#include <iostream>
#include <cassert>
#include <cmath>
#include "physics/atmosphere.hpp"

using namespace ignis;

int main() {
    std::cout << "=== Simple Atmosphere Integration Demo ===" << std::endl;
    
    // Flight scenario: rocket ascending through atmosphere
    std::cout << "\n--- Rocket Atmospheric Flight Profile ---" << std::endl;
    
    // Create all three atmosphere models
    SimpleAtmosphere level1;
    StandardAtmosphere level2;
    DynamicAtmosphere level3(Vector3D(20, 10, 0), 8.0, 0.2);  // 20 m/s east, 10 m/s north wind + gusts
    
    std::cout << "Altitude\tLevel 1\t\tLevel 2\t\tLevel 3" << std::endl;
    std::cout << "[m]\t\tρ [kg/m³]\tρ [kg/m³]\tρ [kg/m³]\tWind [m/s]" << std::endl;
    
    for (double altitude = 0; altitude <= 15000; altitude += 1000) {
        Vector3D position(0, 0, altitude);
        double time = altitude / 100.0;  // Simulate ascending at ~100 m/s
        
        auto state1 = level1.calculate_atmosphere(position, time);
        auto state2 = level2.calculate_atmosphere(position, time);
        auto state3 = level3.calculate_atmosphere(position, time);
        
        std::cout << altitude << "\t\t" 
                  << state1.density << "\t\t" 
                  << state2.density << "\t\t" 
                  << state3.density << "\t\t"
                  << state3.wind_velocity.magnitude() << std::endl;
    }
    
    // Atmosphere effect on drag calculation
    std::cout << "\n--- Atmospheric Drag Effect Analysis ---" << std::endl;
    
    double velocity = 200.0;  // m/s
    double drag_coeff = 0.5;
    double reference_area = 0.0177;  // m² (15cm diameter rocket)
    
    std::cout << "\nDrag force comparison at different altitudes:" << std::endl;
    std::cout << "Altitude\tISA Density\tDrag Force\tReduction" << std::endl;
    std::cout << "[m]\t\t[kg/m³]\t\t[N]\t\t[%]" << std::endl;
    
    double sea_level_drag = 0.0;
    
    for (double altitude = 0; altitude <= 20000; altitude += 2000) {
        Vector3D position(0, 0, altitude);
        auto isa_state = level2.calculate_atmosphere(position);
        
        double drag_force = 0.5 * isa_state.density * velocity * velocity * drag_coeff * reference_area;
        
        if (altitude == 0) {
            sea_level_drag = drag_force;
        }
        
        double reduction_percent = (1.0 - drag_force / sea_level_drag) * 100.0;
        
        std::cout << altitude << "\t\t" 
                  << isa_state.density << "\t\t" 
                  << drag_force << "\t\t"
                  << reduction_percent << "%" << std::endl;
    }
    
    // Wind effect demonstration
    std::cout << "\n--- Wind Effect on Trajectory ---" << std::endl;
    
    Vector3D rocket_velocity(0, 0, 150);  // Vertical ascent at 150 m/s
    double simulation_time = 30.0;  // 30 seconds
    double dt = 1.0;  // 1 second time step
    
    Vector3D pos_no_wind(0, 0, 0);
    Vector3D pos_with_wind(0, 0, 0);
    
    std::cout << "Time\tNo Wind\t\tWith Wind\tDrift" << std::endl;
    std::cout << "[s]\tX [m]\t\tX [m]\t\t[m]" << std::endl;
    
    for (double t = 0; t <= simulation_time; t += dt) {
        // No wind case
        pos_no_wind = pos_no_wind + rocket_velocity * dt;
        
        // With wind case
        auto atm_state = level3.calculate_atmosphere(pos_with_wind, t);
        Vector3D effective_velocity = rocket_velocity + atm_state.wind_velocity;
        pos_with_wind = pos_with_wind + effective_velocity * dt;
        
        double drift = pos_with_wind.x() - pos_no_wind.x();
        
        if (int(t) % 5 == 0) {  // Print every 5 seconds
            std::cout << t << "\t" << pos_no_wind.x() << "\t\t" 
                      << pos_with_wind.x() << "\t\t" << drift << std::endl;
        }
    }
    
    // Temperature effect on sound speed
    std::cout << "\n--- Temperature Effect on Mach Number ---" << std::endl;
    
    double rocket_speed = 300.0;  // m/s
    
    std::cout << "Altitude\tTemperature\tSound Speed\tMach Number" << std::endl;
    std::cout << "[m]\t\t[K]\t\t[m/s]\t\t[-]" << std::endl;
    
    for (double altitude = 0; altitude <= 15000; altitude += 3000) {
        Vector3D position(0, 0, altitude);
        auto isa_state = level2.calculate_atmosphere(position);
        
        // Sound speed calculation: a = sqrt(γ * R * T)
        // For air: γ = 1.4, R = 287 J/(kg·K)
        double sound_speed = std::sqrt(1.4 * 287.0 * isa_state.temperature);
        double mach_number = rocket_speed / sound_speed;
        
        std::cout << altitude << "\t\t" 
                  << isa_state.temperature << "\t\t" 
                  << sound_speed << "\t\t"
                  << mach_number << std::endl;
    }
    
    std::cout << "\n=== Atmosphere integration demo completed! ===" << std::endl;
    return 0;
}
