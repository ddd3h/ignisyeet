#ifndef OUTPUT_H
#define OUTPUT_H

#define CSV_FILE "output.csv"

typedef enum {
    FORMAT_ECEF,  // 出力形式: X, Y, Z (ECEF)
    FORMAT_LATLON // 出力形式: 緯度, 経度, 高度
} OutputFormat;

void initialize_csv(OutputFormat format);
void save_to_csv(OutputFormat format, double time, double x, double y, double z, double vz);

#endif
