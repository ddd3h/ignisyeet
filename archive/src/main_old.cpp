#include "rocket.hpp"
#include "parameter.hpp"
#include "output.hpp"
#include <iostream>
#include <iomanip>
#include <thread>
#include <chrono>

void print_progress(double progress) {
    int bar_width = 50;  // バーの長さ
    int pos = static_cast<int>(progress * bar_width);  // 進行状況の割合

    std::cout << "\r[";
    for (int i = 0; i < bar_width; i++) {
        if (i < pos) std::cout << "=";
        else if (i == pos) std::cout << ">";
        else std::cout << " ";
    }
    std::cout << "] " << std::fixed << std::setprecision(1) << (progress * 100.0) << "%" << std::flush;

    // CPU 使用率を抑えるために少しスリープ（過度な更新を防止）
    std::this_thread::sleep_for(std::chrono::milliseconds(50));
}

int main(int argc, char* argv[]) {
    if (argc < 2) {
        std::cerr << "Usage: " << argv[0] << " <parameter file>\n";
        return 1;
    }

    Parameter param(argv[1]);
    if (!param.loadParameters()) {
        std::cerr << "Error loading parameters!\n";
        return 1;
    }

    std::cout << "\n🚀 Rocket Simulation Started 🚀\n";
    std::cout << "Launch Site: Lat: " << param.rocket.Xe[0]
              << ", Lon: " << param.rocket.Xe[1]
              << ", Alt: " << param.rocket.Xe[2] << " m\n";

    initialize_csv(param.output_format);

    double time = 0.0, max_time = 50.0;

    while (param.rocket.Xe[2] > 0 && time < max_time) {
        param.rocket.thrust = param.rocket.compute_thrust(time);
        param.rocket.update(param.environment, param.environment.dt);
        save_to_csv(param.output_format, time, param.rocket.Xe[0], param.rocket.Xe[1], param.rocket.Xe[2], param.rocket.Ve[2]);

        // 🏁 **プログレスバーの更新を適切に配置**
        if (static_cast<int>(time) % 1 == 0) {  // 1秒ごとに更新
            print_progress(time / max_time);
        }

        time += param.environment.dt;
    }

    std::cout << "\n🏁 Rocket has landed! 🏁\n";
    return 0;
}
