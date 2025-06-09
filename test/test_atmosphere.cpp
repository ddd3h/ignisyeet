#include <iostream>
#include <cassert>
#include <cmath>
#include "physics/atmosphere.hpp"

using namespace ignis;

int main() {
    std::cout << "=== Atmosphere Model Comprehensive Test ===" << std::endl;
    
    // Test Level 1: Simple Atmosphere
    std::cout << "\n--- Level 1: Simple Atmosphere Test ---" << std::endl;
    SimpleAtmosphere simple_atm(1.225, 288.15, 101325.0);
    
    Vector3D pos1(0, 0, 0);      // Sea level
    Vector3D pos2(0, 0, 1000);   // 1km altitude
    Vector3D pos3(0, 0, 10000);  // 10km altitude
    
    auto state1 = simple_atm.calculate_atmosphere(pos1);
    auto state2 = simple_atm.calculate_atmosphere(pos2);
    auto state3 = simple_atm.calculate_atmosphere(pos3);
    
    assert(std::abs(state1.density - 1.225) < 1e-9);
    assert(std::abs(state2.density - 1.225) < 1e-9);
    assert(std::abs(state3.density - 1.225) < 1e-9);
    std::cout << "✓ Simple atmosphere: ρ = " << state1.density << " kg/m³ (constant)" << std::endl;
    std::cout << "✓ Simple atmosphere: T = " << state1.temperature << " K (constant)" << std::endl;
    std::cout << "✓ Simple atmosphere: P = " << state1.pressure << " Pa (constant)" << std::endl;
    
    // Test Level 2: ISA Standard Atmosphere
    std::cout << "\n--- Level 2: ISA Standard Atmosphere Test ---" << std::endl;
    StandardAtmosphere isa_atm;
    
    auto isa1 = isa_atm.calculate_atmosphere(pos1);
    auto isa2 = isa_atm.calculate_atmosphere(pos2);
    auto isa3 = isa_atm.calculate_atmosphere(pos3);
    
    std::cout << "✓ Sea level: ρ = " << isa1.density << " kg/m³, T = " << isa1.temperature << " K" << std::endl;
    std::cout << "✓ 1km altitude: ρ = " << isa2.density << " kg/m³, T = " << isa2.temperature << " K" << std::endl;
    std::cout << "✓ 10km altitude: ρ = " << isa3.density << " kg/m³, T = " << isa3.temperature << " K" << std::endl;
    
    // Verify atmospheric properties decrease with altitude
    assert(isa2.density < isa1.density);
    assert(isa3.density < isa2.density);
    assert(isa2.temperature < isa1.temperature);
    assert(isa3.temperature < isa2.temperature);
    
    // Test with constant wind
    Vector3D wind(10.0, 5.0, 0.0);  // 10 m/s east, 5 m/s north
    StandardAtmosphere isa_wind(wind);
    auto isa_wind_state = isa_wind.calculate_atmosphere(pos1);
    
    assert(std::abs(isa_wind_state.wind_velocity.x() - 10.0) < 1e-9);
    assert(std::abs(isa_wind_state.wind_velocity.y() - 5.0) < 1e-9);
    std::cout << "✓ Wind velocity: " << isa_wind_state.wind_velocity.x() << " m/s east, " 
              << isa_wind_state.wind_velocity.y() << " m/s north" << std::endl;
    
    // Test Level 3: Dynamic Atmosphere
    std::cout << "\n--- Level 3: Dynamic Atmosphere Test ---" << std::endl;
    DynamicAtmosphere dynamic_atm(wind, 5.0, 0.1);  // 5 m/s gust intensity, 0.1 Hz frequency
    
    auto dyn1 = dynamic_atm.calculate_atmosphere(pos1, 0.0);
    auto dyn2 = dynamic_atm.calculate_atmosphere(pos1, 5.0);  // 5 seconds later
    auto dyn3 = dynamic_atm.calculate_atmosphere(pos2, 0.0);  // Different altitude
    
    std::cout << "✓ Dynamic atmosphere at t=0s: wind = " << dyn1.wind_velocity.magnitude() << " m/s" << std::endl;
    std::cout << "✓ Dynamic atmosphere at t=5s: wind = " << dyn2.wind_velocity.magnitude() << " m/s" << std::endl;
    std::cout << "✓ Dynamic atmosphere at 1km: wind = " << dyn3.wind_velocity.magnitude() << " m/s" << std::endl;
    
    // ISA standard verification
    std::cout << "\n--- ISA Standard Values Verification ---" << std::endl;
    
    // Sea level standard values
    assert(std::abs(isa1.temperature - 288.15) < 0.1);  // 15°C
    assert(std::abs(isa1.pressure - 101325.0) < 100.0); // Standard pressure
    assert(std::abs(isa1.density - 1.225) < 0.01);      // Standard density
    
    // 11km altitude (tropopause)
    Vector3D pos_tropopause(0, 0, 11000);
    auto isa_tropo = isa_atm.calculate_atmosphere(pos_tropopause);
    
    std::cout << "✓ Tropopause (11km): T = " << isa_tropo.temperature << " K, ρ = " << isa_tropo.density << " kg/m³" << std::endl;
    assert(std::abs(isa_tropo.temperature - 216.65) < 1.0);  // -56.5°C
    
    // Density reduction check
    double density_ratio_1km = isa2.density / isa1.density;
    double density_ratio_10km = isa3.density / isa1.density;
    
    std::cout << "✓ Density ratio at 1km: " << density_ratio_1km << " (expected ~0.89)" << std::endl;
    std::cout << "✓ Density ratio at 10km: " << density_ratio_10km << " (expected ~0.34)" << std::endl;
    
    assert(density_ratio_1km > 0.85 && density_ratio_1km < 0.95);
    assert(density_ratio_10km > 0.30 && density_ratio_10km < 0.40);
    
    std::cout << "\n=== All atmosphere tests completed successfully! ===" << std::endl;
    return 0;
}
