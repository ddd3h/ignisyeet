#include <iostream>
#include <cassert>
#include <cmath>
#include "physics/vector3d.hpp"

using namespace IgnisYeet::Physics;

int main() {
    std::cout << "=== IgnisYeet Minimal Integration Test ===" << std::endl;
    
    // Test Vector3D
    Vector3D v1(1.0, 2.0, 3.0);
    Vector3D v2(4.0, 5.0, 6.0);
    Vector3D v3 = v1 + v2;
    
    assert(std::abs(v3.x() - 5.0) < 1e-9);
    assert(std::abs(v3.y() - 7.0) < 1e-9);
    assert(std::abs(v3.z() - 9.0) < 1e-9);
    std::cout << "✓ Vector3D operations test passed" << std::endl;
    
    // Test Vector3D magnitude
    Vector3D v4(3.0, 4.0, 0.0);
    assert(std::abs(v4.magnitude() - 5.0) < 1e-9);
    std::cout << "✓ Vector3D magnitude test passed" << std::endl;
    
    // Test Vector3D normalization
    Vector3D v5 = v4.normalized();
    assert(std::abs(v5.magnitude() - 1.0) < 1e-9);
    std::cout << "✓ Vector3D normalization test passed" << std::endl;
    
    std::cout << "=== All minimal tests completed successfully! ===" << std::endl;
    return 0;
}
