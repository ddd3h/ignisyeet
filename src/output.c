#include "output.h"
#include <stdio.h>
#include <math.h>

// 地球の半径 (WGS84)
#define EARTH_RADIUS 6378137.0
#define EARTH_FLATTENING (1.0 / 298.257223563)  // 扁平率

// CSVヘッダーを書き込む関数
void initialize_csv(OutputFormat format) {
    FILE *file = fopen(CSV_FILE, "w");
    if (file == NULL) {
        printf("Error: Cannot open output file.\n");
        return;
    }

    if (format == FORMAT_ECEF) {
        fprintf(file, "time,x,y,z,vz\n");
    } else {
        fprintf(file, "time,latitude,longitude,altitude,vz\n");
    }

    fclose(file);
}

// 緯度・経度・高度（海抜）を ECEF に変換する
void latlon_to_ecef(double lat, double lon, double altitude, double *x, double *y, double *z) {
    double a = EARTH_RADIUS; 
    double f = EARTH_FLATTENING;
    double b = a * (1 - f); // 楕円体の短軸

    double e2 = (a * a - b * b) / (a * a); // 第一離心率の二乗
    double sin_lat = sin(lat * M_PI / 180.0);
    double cos_lat = cos(lat * M_PI / 180.0);
    double sin_lon = sin(lon * M_PI / 180.0);
    double cos_lon = cos(lon * M_PI / 180.0);

    double N = a / sqrt(1 - e2 * sin_lat * sin_lat);
    *x = (N + altitude) * cos_lat * cos_lon;
    *y = (N + altitude) * cos_lat * sin_lon;
    *z = ((b * b / (a * a)) * N + altitude) * sin_lat;
}

// ECEF → 緯度経度（海抜高度基準）
void ecef_to_latlon(double x, double y, double z, double *lat, double *lon, double *alt) {
    double a = EARTH_RADIUS; 
    double f = EARTH_FLATTENING;
    double b = a * (1 - f); // 楕円体の短軸

    double e2 = (a * a - b * b) / (a * a); // 第一離心率の二乗
    double ep2 = (a * a - b * b) / (b * b); // 第二離心率の二乗

    *lon = atan2(y, x);
    
    double p = sqrt(x * x + y * y);
    double theta = atan2(z * a, p * b);
    *lat = atan2(z + ep2 * b * pow(sin(theta), 3), p - e2 * a * pow(cos(theta), 3));

    double N = a / sqrt(1 - e2 * pow(sin(*lat), 2));
    *alt = p / cos(*lat) - N; // 海抜高度

    // ラジアン → 度
    *lat *= (180.0 / M_PI);
    *lon *= (180.0 / M_PI);
}

// シミュレーション結果をCSVに追記する関数
void save_to_csv(OutputFormat format, double time, double x, double y, double z, double vz) {
    FILE *file = fopen(CSV_FILE, "a");
    if (file == NULL) {
        printf("Error: Cannot open output file.\n");
        return;
    }

    if (format == FORMAT_ECEF) {
        fprintf(file, "%.2f,%.2f,%.2f,%.2f,%.2f\n", time, x, y, z, vz);
    } else {
        double lat, lon, altitude;
        ecef_to_latlon(x, y, z, &lat, &lon, &altitude);
        fprintf(file, "%.2f,%.6f,%.6f,%.2f,%.2f\n", time, lat, lon, z, vz);  // `z` を海抜高度として出力
    }

    fclose(file);
}
