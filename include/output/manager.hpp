#ifndef IGNISYEET_OUTPUT_MANAGER_HPP
#define IGNISYEET_OUTPUT_MANAGER_HPP

#include "../physics/state.hpp"
#include "../parameter.hpp"
#include <string>
#include <vector>
#include <memory>
#include <fstream>
#include <unordered_map>

namespace IgnisYeet::Output {

/**
 * @brief Output record for simulation data
 */
struct OutputRecord {
    double time;
    Physics::RigidBodyState state;
    Physics::EnvironmentState environment;
    double mass;
    double thrust;
    
    // Missing fields for manager.cpp compatibility
    double altitude;
    double velocity_magnitude;
    double mach_number;
    Physics::Vector3D position;
    Physics::Vector3D velocity;
    
    OutputRecord() : time(0.0), mass(0.0), thrust(0.0), altitude(0.0), velocity_magnitude(0.0), mach_number(0.0) {}
};

/**
 * @brief Output data record for a single time step
 */
struct DataRecord {
    // Time
    double time;
    
    // Position and velocity
    double x, y, z;                    // Position [m]
    double vx, vy, vz;                 // Velocity [m/s]
    double ax, ay, az;                 // Acceleration [m/s²]
    
    // Orientation (Euler angles)
    double roll, pitch, yaw;           // Orientation [rad]
    double roll_rate, pitch_rate, yaw_rate; // Angular rates [rad/s]
    
    // Flight parameters
    double altitude;                   // Altitude [m]
    double velocity_magnitude;         // Speed [m/s]
    double mach_number;               // Mach number
    double dynamic_pressure;          // Dynamic pressure [Pa]
    
    // Mass and forces
    double mass;                      // Current mass [kg]
    double thrust;                    // Thrust [N]
    double drag;                      // Drag force [N]
    
    // Environmental
    double air_density;               // Air density [kg/m³]
    double temperature;               // Temperature [K]
    double wind_speed;                // Wind speed [m/s]
    
    // Geographic coordinates (if applicable)
    double latitude, longitude;       // Geographic coordinates [deg]
    
    DataRecord();
    DataRecord(const Physics::RigidBodyState& state, 
               const Physics::EnvironmentState& env,
               double thrust_force);
};

/**
 * @brief Base class for output writers
 */
class OutputWriter {
public:
    virtual ~OutputWriter() = default;
    
    // Initialize output file/stream
    virtual bool initialize(const std::string& filename) = 0;
    
    // Write header (column names, metadata, etc.)
    virtual bool write_header(const std::unordered_map<std::string, std::string>& metadata) = 0;
    
    // Write a single data record
    virtual bool write_record(const DataRecord& record) = 0;
    
    // Write multiple records at once
    virtual bool write_records(const std::vector<DataRecord>& records);
    
    // Finalize and close output
    virtual bool finalize() = 0;
    
    // Close output (alias for finalize)
    virtual bool close() { return finalize(); }
    
    // Get file extension
    virtual std::string extension() const = 0;
    
    // Get format name
    virtual std::string format_name() const = 0;
};

/**
 * @brief CSV output writer
 */
class CSVWriter : public OutputWriter {
private:
    std::ofstream file_;
    std::string delimiter_;
    bool header_written_;
    
public:
    CSVWriter(const std::string& delimiter = ",");
    ~CSVWriter();
    
    bool initialize(const std::string& filename) override;
    bool write_header(const std::unordered_map<std::string, std::string>& metadata) override;
    bool write_record(const DataRecord& record) override;
    bool finalize() override;
    
    std::string extension() const override { return ".csv"; }
    std::string format_name() const override { return "CSV"; }
    
private:
    void write_csv_header();
    std::string format_record(const DataRecord& record) const;
};

/**
 * @brief JSON Lines output writer (each record as a JSON object)
 */
class JSONLWriter : public OutputWriter {
private:
    std::ofstream file_;
    bool first_record_;
    
public:
    JSONLWriter();
    ~JSONLWriter();
    
    bool initialize(const std::string& filename) override;
    bool write_header(const std::unordered_map<std::string, std::string>& metadata) override;
    bool write_record(const DataRecord& record) override;
    bool finalize() override;
    
    std::string extension() const override { return ".jsonl"; }
    std::string format_name() const override { return "JSON Lines"; }
    
private:
    std::string format_record(const DataRecord& record) const;
};

/**
 * @brief Binary output writer (placeholder for future HDF5/Feather support)
 */
class BinaryWriter : public OutputWriter {
private:
    std::ofstream file_;
    std::vector<DataRecord> buffer_;
    
public:
    BinaryWriter();
    ~BinaryWriter();
    
    bool initialize(const std::string& filename) override;
    bool write_header(const std::unordered_map<std::string, std::string>& metadata) override;
    bool write_record(const DataRecord& record) override;
    bool finalize() override;
    
    std::string extension() const override { return ".bin"; }
    std::string format_name() const override { return "Binary"; }
};

/**
 * @brief Output manager coordinating multiple output formats
 */
class OutputManager {
private:
    std::vector<std::unique_ptr<OutputWriter>> writers_;
    std::vector<DataRecord> data_buffer_;
    size_t buffer_size_;
    std::string output_directory_;
    std::string base_filename_;
    std::unordered_map<std::string, std::string> metadata_;
    
    // Missing member variables for manager.cpp compatibility
    OutputConfig config_;
    std::vector<OutputRecord> output_buffer_;
    size_t records_written_;
    
    struct OutputStatistics {
        size_t total_records;
        double max_altitude;
        double max_velocity;
        double max_mach;
        double simulation_start_time;
        double simulation_end_time;
        Physics::Vector3D final_position;
        Physics::Vector3D final_velocity;
        double final_mass;
        
        OutputStatistics() : total_records(0), max_altitude(0.0), max_velocity(0.0), max_mach(0.0),
                            simulation_start_time(0.0), simulation_end_time(0.0), final_mass(0.0) {}
    };
    OutputStatistics statistics_;
    
public:
    OutputManager(const OutputConfig& config);
    OutputManager(const Parameter& params);  // Add Parameter constructor
    ~OutputManager();
    
    // Non-copyable but movable
    OutputManager(const OutputManager&) = delete;
    OutputManager& operator=(const OutputManager&) = delete;
    OutputManager(OutputManager&&) = default;
    OutputManager& operator=(OutputManager&&) = default;
    
    // Initialize all writers
    bool initialize();
    void reset();  // Add reset method
    void initialize_from_parameters(const Parameter& params);  // Add parameter initialization
    
    // Add metadata
    void add_metadata(const std::string& key, const std::string& value);
    void add_metadata(const std::unordered_map<std::string, std::string>& metadata);
    
    // Write data
    bool write_record(const DataRecord& record);
    bool write_record(const OutputRecord& record);  // Add OutputRecord version
    bool write_records(const std::vector<DataRecord>& records);
    
    // Flush buffer to files
    bool flush();
    
    // Finalize all outputs
    bool finalize();
    
    // Missing methods for manager.cpp compatibility  
    void update_statistics(const OutputRecord& record);
    bool flush_buffer();
    void write_statistics_file();
    
    // Buffer management
    void set_buffer_size(size_t size) { buffer_size_ = size; }
    size_t buffer_size() const { return buffer_size_; }
    size_t buffer_count() const { return data_buffer_.size(); }
    
    // File management
    std::string get_output_path(const std::string& extension) const;
    
    // Statistics
    struct Statistics {
        size_t total_records;
        double simulation_time;
        double max_altitude;
        double max_velocity;
        double max_acceleration;
        double max_mach;
        double flight_time;
        double burnout_time;
        double apogee_time;
        size_t integration_steps;
        double average_step_size;
        bool recovery_deployed;
        double recovery_deployment_time;
        double recovery_deployment_altitude;
        double simulation_start_time;
        double simulation_end_time;
        Physics::Vector3D final_position;
        Physics::Vector3D final_velocity;
        double final_mass;
        
        Statistics() : total_records(0), simulation_time(0.0), 
                      max_altitude(0.0), max_velocity(0.0), 
                      max_acceleration(0.0), max_mach(0.0),
                      flight_time(0.0), burnout_time(0.0), apogee_time(0.0),
                      integration_steps(0), average_step_size(0.0),
                      recovery_deployed(false), recovery_deployment_time(0.0),
                      recovery_deployment_altitude(0.0), simulation_start_time(0.0),
                      simulation_end_time(0.0), final_mass(0.0) {}
    };
    
    Statistics compute_statistics() const;
    bool write_statistics(const Statistics& stats) const;
    
private:
    void create_writers(const OutputConfig& config);
    void create_output_directory();
};

/**
 * @brief Factory for creating output writers
 */
class OutputWriterFactory {
public:
    static std::unique_ptr<OutputWriter> create(OutputFormat format);
    static std::vector<std::unique_ptr<OutputWriter>> create_multiple(
        const std::vector<OutputFormat>& formats);
};

} // namespace IgnisYeet::Output

#endif // IGNISYEET_OUTPUT_MANAGER_HPP
