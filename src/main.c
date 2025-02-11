#include "rocket.h"
#include "parameter.h"
#include "output.h"
#include <stdio.h>
#include <stdlib.h>

// プログレスバーを表示する関数
void print_progress(double progress) {
    int bar_width = 50;  // バーの長さ
    int pos = (int)(progress * bar_width);  // 進行状況の割合

    printf("\r[");
    for (int i = 0; i < bar_width; i++) {
        if (i < pos) printf("=");
        else if (i == pos) printf(">");
        else printf(" ");
    }
    printf("] %.1f%%", progress * 100);
    fflush(stdout);
}

int main(int argc, char *argv[]) {

    if (argc < 2) {
        printf("Usage: %s <parameter file>\n", argv[0]);
        return 1;
    }

    const char *param_file = argv[1];  // コマンドライン引数からパラメータファイル名を取得

    Rocket rocket;
    Environment env;
    OutputFormat format;

    if (!read_parameters(&rocket, &env, &format, param_file)) {
        printf("Error: Failed to read parameters from %s\n", param_file);
        return 1;
    }

    printf("\n🚀 Rocket Simulation Started 🚀\n");
    printf("Initial Conditions:\n");
    printf("  Position (X: %.2f, Y: %.2f, Z: %.2f)\n", rocket.Xe[0], rocket.Xe[1], rocket.Xe[2]);
    printf("  Velocity (Vx: %.2f, Vy: %.2f, Vz: %.2f)\n", rocket.Ve[0], rocket.Ve[1], rocket.Ve[2]);
    printf("  Mass: %.2f kg, Thrust: %.2f N, Thrust Duration: %.2f s\n", rocket.mass, rocket.thrust, rocket.thrust_duration);
    printf("  Output format: %s\n\n", (format == FORMAT_ECEF) ? "ECEF" : "LATLON");

    initialize_csv(format);

    double time = 0.0;
    double total_time = 50.0;  // 目安となる最大シミュレーション時間

    while (rocket.Xe[2] > 0 && time < total_time) {
        rocket.thrust = compute_thrust(time, &rocket);
        update_rocket_leapfrog(&rocket, &env, env.dt);

        // CSVにデータを保存
        save_to_csv(format, time, rocket.Xe[0], rocket.Xe[1], rocket.Xe[2], rocket.Ve[2]);

        // プログレスバーの更新
        print_progress(time / total_time);

        time += env.dt;
    }

    // シミュレーション終了メッセージ
    printf("\n\n🏁 Rocket has landed! 🏁\n");
    printf("Final Conditions:\n");
    printf("  Position (X: %.2f, Y: %.2f, Z: %.2f)\n", rocket.Xe[0], rocket.Xe[1], rocket.Xe[2]);
    printf("  Velocity (Vx: %.2f, Vy: %.2f, Vz: %.2f)\n", rocket.Ve[0], rocket.Ve[1], rocket.Ve[2]);
    printf("Simulation complete. Results saved to 'output.csv'.\n");

    return 0;
}
