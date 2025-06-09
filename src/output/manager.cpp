#include "../../include/output/manager.hpp"
#include "../../include/parameter.hpp"
#include <iostream>
#include <fstream>
#include <sstream>
#include <iomanip>
#include <algorithm>
#include <cmath>

namespace IgnisYeet::Output {

OutputManager::OutputManager(const Parameters& params) {
    initialize_from_parameters(params);
}

bool OutputManager::initialize() {
    try {
        // Create output directory if it doesn't exist
        create_output_directory();
        
        // Initialize writers based on configuration
        for (auto format : config_.output_formats) {
            auto writer = create_writer(format);
            if (writer && writer->initialize(config_.base_filename)) {
                writers_.push_back(std::move(writer));
            }
        }
        
        if (writers_.empty()) {
            std::cerr << "No output writers could be initialized" << std::endl;
            return false;
        }
        
        // Clear statistics
        statistics_ = OutputStatistics{};
        
        return true;
        
    } catch (const std::exception& e) {
        std::cerr << "Output manager initialization failed: " << e.what() << std::endl;
        return false;
    }
}

void OutputManager::reset() {
    // Close all writers
    for (auto& writer : writers_) {
        writer->finalize();
    }
    writers_.clear();
    
    // Clear buffer and statistics
    output_buffer_.clear();
    statistics_ = OutputStatistics{};
    records_written_ = 0;
}

bool OutputManager::write_record(const OutputRecord& record) {
    try {
        // Add to buffer if buffering is enabled
        if (config_.enable_buffering) {
            output_buffer_.push_back(record);
            
            // Flush buffer if it's full
            if (output_buffer_.size() >= config_.buffer_size) {
                return flush_buffer();
            }
        } else {
            // Write directly
            return write_record_to_all_writers(record);
        }
        
        return true;
        
    } catch (const std::exception& e) {
        std::cerr << "Failed to write record: " << e.what() << std::endl;
        return false;
    }
}

bool OutputManager::flush() {
    if (config_.enable_buffering && !output_buffer_.empty()) {
        return flush_buffer();
    }
    return true;
}

bool OutputManager::finalize() {
    try {
        // Flush any remaining buffered data
        if (!flush()) {
            return false;
        }
        
        // Finalize all writers
        for (auto& writer : writers_) {
            if (!writer->finalize()) {
                return false;
            }
        }
        
        // Write summary statistics if enabled
        if (config_.write_statistics) {
            write_statistics_file();
        }
        
        return true;
        
    } catch (const std::exception& e) {
        std::cerr << "Output finalization failed: " << e.what() << std::endl;
        return false;
    }
}

void OutputManager::update_statistics(const OutputRecord& record) {
    statistics_.total_records++;
    
    // Update position statistics
    statistics_.max_altitude = std::max(statistics_.max_altitude, record.altitude);
    statistics_.max_velocity = std::max(statistics_.max_velocity, record.velocity_magnitude);
    statistics_.max_mach = std::max(statistics_.max_mach, record.mach_number);
    
    // Update time range
    statistics_.simulation_start_time = std::min(statistics_.simulation_start_time, record.time);
    statistics_.simulation_end_time = std::max(statistics_.simulation_end_time, record.time);
    
    // Track final values
    statistics_.final_position = record.position;
    statistics_.final_velocity = record.velocity;
    statistics_.final_mass = record.mass;
}

void OutputManager::initialize_from_parameters(const Parameters& params) {
    // Output configuration
    config_.base_filename = params.output.filename;
    config_.output_directory = params.output.directory;
    config_.enable_buffering = params.output.enable_buffering;
    config_.buffer_size = params.output.buffer_size;
    config_.write_statistics = params.output.write_statistics;
    config_.coordinate_system = params.output.coordinate_system;
    
    // Output formats
    config_.output_formats.clear();
    if (params.output.enable_csv) {
        config_.output_formats.push_back(OutputFormat::CSV);
    }
    if (params.output.enable_json) {
        config_.output_formats.push_back(OutputFormat::JSON);
    }
    if (params.output.enable_binary) {
        config_.output_formats.push_back(OutputFormat::BINARY);
    }
    if (params.output.enable_hdf5) {
        config_.output_formats.push_back(OutputFormat::HDF5);
    }
    
    // Default to CSV if no formats specified
    if (config_.output_formats.empty()) {
        config_.output_formats.push_back(OutputFormat::CSV);
    }
}

std::unique_ptr<OutputWriter> OutputManager::create_writer(OutputFormat format) {
    switch (format) {
        case OutputFormat::CSV:
            return std::make_unique<CSVWriter>(config_);
        case OutputFormat::JSON:
            return std::make_unique<JSONWriter>(config_);
        case OutputFormat::BINARY:
            return std::make_unique<BinaryWriter>(config_);
        case OutputFormat::HDF5:
            return std::make_unique<HDF5Writer>(config_);
        default:
            return nullptr;
    }
}

void OutputManager::create_output_directory() {
    // This is a simplified implementation
    // In a real implementation, you would use std::filesystem
    // For now, assume directory exists or create manually
}

bool OutputManager::flush_buffer() {
    if (output_buffer_.empty()) {
        return true;
    }
    
    try {
        for (const auto& record : output_buffer_) {
            if (!write_record_to_all_writers(record)) {
                return false;
            }
        }
        
        output_buffer_.clear();
        return true;
        
    } catch (const std::exception& e) {
        std::cerr << "Buffer flush failed: " << e.what() << std::endl;
        return false;
    }
}

bool OutputManager::write_record_to_all_writers(const OutputRecord& record) {
    bool success = true;
    
    for (auto& writer : writers_) {
        if (!writer->write_record(record)) {
            success = false;
        }
    }
    
    if (success) {
        update_statistics(record);
        records_written_++;
    }
    
    return success;
}

void OutputManager::write_statistics_file() {
    try {
        std::string stats_filename = config_.output_directory + "/" + 
                                   config_.base_filename + "_statistics.txt";
        
        std::ofstream file(stats_filename);
        if (!file.is_open()) {
            std::cerr << "Could not create statistics file: " << stats_filename << std::endl;
            return;
        }
        
        file << "IgnisYeet Simulation Statistics\n";
        file << "================================\n\n";
        
        file << "Simulation Overview:\n";
        file << "  Total Records: " << statistics_.total_records << "\n";
        file << "  Start Time: " << statistics_.simulation_start_time << " s\n";
        file << "  End Time: " << statistics_.simulation_end_time << " s\n";
        file << "  Duration: " << (statistics_.simulation_end_time - statistics_.simulation_start_time) << " s\n\n";
        
        file << "Maximum Values:\n";
        file << "  Altitude: " << statistics_.max_altitude << " m\n";
        file << "  Velocity: " << statistics_.max_velocity << " m/s\n";
        file << "  Mach Number: " << statistics_.max_mach << "\n\n";
        
        file << "Final State:\n";
        file << "  Position: (" << statistics_.final_position.x() << ", " 
             << statistics_.final_position.y() << ", " << statistics_.final_position.z() << ") m\n";
        file << "  Velocity: (" << statistics_.final_velocity.x() << ", "
             << statistics_.final_velocity.y() << ", " << statistics_.final_velocity.z() << ") m/s\n";
        file << "  Mass: " << statistics_.final_mass << " kg\n";
        
        file.close();
        
    } catch (const std::exception& e) {
        std::cerr << "Failed to write statistics file: " << e.what() << std::endl;
    }
}

// CSV Writer Implementation
CSVWriter::CSVWriter(const OutputConfig& config) : config_(config) {}

bool CSVWriter::initialize(const std::string& base_filename) {
    filename_ = config_.output_directory + "/" + base_filename + ".csv";
    
    file_.open(filename_);
    if (!file_.is_open()) {
        std::cerr << "Could not create CSV file: " << filename_ << std::endl;
        return false;
    }
    
    // Write header
    write_header();
    
    return true;
}

bool CSVWriter::write_record(const OutputRecord& record) {
    try {
        file_ << std::fixed << std::setprecision(6);
        
        if (config_.coordinate_system == CoordinateSystem::ECEF) {
            file_ << record.time << ","
                  << record.position.x() << ","
                  << record.position.y() << ","
                  << record.position.z() << ","
                  << record.velocity.x() << ","
                  << record.velocity.y() << ","
                  << record.velocity.z() << ","
                  << record.velocity_magnitude << ","
                  << record.mach_number << ","
                  << record.mass << "\n";
        } else {
            // Convert to latitude/longitude
            auto [lat, lon, alt] = convert_to_geodetic(record.position);
            
            file_ << record.time << ","
                  << lat << ","
                  << lon << ","
                  << alt << ","
                  << record.velocity_magnitude << ","
                  << record.mach_number << ","
                  << record.mass << "\n";
        }
        
        records_written_++;
        return true;
        
    } catch (const std::exception& e) {
        std::cerr << "CSV write error: " << e.what() << std::endl;
        return false;
    }
}

bool CSVWriter::finalize() {
    if (file_.is_open()) {
        file_.close();
    }
    return true;
}

void CSVWriter::write_header() {
    if (config_.coordinate_system == CoordinateSystem::ECEF) {
        file_ << "time,x,y,z,vx,vy,vz,velocity_magnitude,mach_number,mass\n";
    } else {
        file_ << "time,latitude,longitude,altitude,velocity_magnitude,mach_number,mass\n";
    }
}

std::tuple<double, double, double> CSVWriter::convert_to_geodetic(
    const Physics::Vector3D& position) const {
    
    // Simplified ECEF to geodetic conversion
    // This is a placeholder - real implementation would use proper geodetic conversion
    
    double x = position.x();
    double y = position.y();
    double z = position.z();
    
    // Simple approximation for small distances
    const double earth_radius = 6371000.0;
    
    double longitude = std::atan2(y, x) * 180.0 / M_PI;
    double latitude = std::asin(z / std::sqrt(x*x + y*y + z*z)) * 180.0 / M_PI;
    double altitude = std::sqrt(x*x + y*y + z*z) - earth_radius;
    
    return {latitude, longitude, altitude};
}

// JSON Writer Implementation
JSONWriter::JSONWriter(const OutputConfig& config) : config_(config), first_record_(true) {}

bool JSONWriter::initialize(const std::string& base_filename) {
    filename_ = config_.output_directory + "/" + base_filename + ".json";
    
    file_.open(filename_);
    if (!file_.is_open()) {
        std::cerr << "Could not create JSON file: " << filename_ << std::endl;
        return false;
    }
    
    // Start JSON array
    file_ << "{\n";
    file_ << "  \"simulation_data\": [\n";
    
    return true;
}

bool JSONWriter::write_record(const OutputRecord& record) {
    try {
        if (!first_record_) {
            file_ << ",\n";
        } else {
            first_record_ = false;
        }
        
        file_ << "    {\n";
        file_ << "      \"time\": " << record.time << ",\n";
        file_ << "      \"position\": {\"x\": " << record.position.x() 
              << ", \"y\": " << record.position.y() 
              << ", \"z\": " << record.position.z() << "},\n";
        file_ << "      \"velocity\": {\"x\": " << record.velocity.x()
              << ", \"y\": " << record.velocity.y()
              << ", \"z\": " << record.velocity.z() << "},\n";
        file_ << "      \"velocity_magnitude\": " << record.velocity_magnitude << ",\n";
        file_ << "      \"mach_number\": " << record.mach_number << ",\n";
        file_ << "      \"mass\": " << record.mass << "\n";
        file_ << "    }";
        
        records_written_++;
        return true;
        
    } catch (const std::exception& e) {
        std::cerr << "JSON write error: " << e.what() << std::endl;
        return false;
    }
}

bool JSONWriter::finalize() {
    if (file_.is_open()) {
        file_ << "\n  ]\n";
        file_ << "}\n";
        file_.close();
    }
    return true;
}

// Binary Writer Implementation
BinaryWriter::BinaryWriter(const OutputConfig& config) : config_(config) {}

bool BinaryWriter::initialize(const std::string& base_filename) {
    filename_ = config_.output_directory + "/" + base_filename + ".bin";
    
    file_.open(filename_, std::ios::binary);
    if (!file_.is_open()) {
        std::cerr << "Could not create binary file: " << filename_ << std::endl;
        return false;
    }
    
    // Write header with version and format info
    uint32_t version = 1;
    uint32_t record_size = sizeof(BinaryRecord);
    
    file_.write(reinterpret_cast<const char*>(&version), sizeof(version));
    file_.write(reinterpret_cast<const char*>(&record_size), sizeof(record_size));
    
    return true;
}

bool BinaryWriter::write_record(const OutputRecord& record) {
    try {
        BinaryRecord bin_record;
        bin_record.time = record.time;
        bin_record.position_x = record.position.x();
        bin_record.position_y = record.position.y();
        bin_record.position_z = record.position.z();
        bin_record.velocity_x = record.velocity.x();
        bin_record.velocity_y = record.velocity.y();
        bin_record.velocity_z = record.velocity.z();
        bin_record.velocity_magnitude = record.velocity_magnitude;
        bin_record.mach_number = record.mach_number;
        bin_record.mass = record.mass;
        
        file_.write(reinterpret_cast<const char*>(&bin_record), sizeof(bin_record));
        
        records_written_++;
        return true;
        
    } catch (const std::exception& e) {
        std::cerr << "Binary write error: " << e.what() << std::endl;
        return false;
    }
}

bool BinaryWriter::finalize() {
    if (file_.is_open()) {
        file_.close();
    }
    return true;
}

// HDF5 Writer Implementation (Placeholder)
HDF5Writer::HDF5Writer(const OutputConfig& config) : config_(config) {}

bool HDF5Writer::initialize(const std::string& base_filename) {
    // Placeholder - real HDF5 implementation would require HDF5 library
    filename_ = config_.output_directory + "/" + base_filename + ".h5";
    std::cerr << "HDF5 writer not implemented - would create: " << filename_ << std::endl;
    return false; // Not implemented
}

bool HDF5Writer::write_record(const OutputRecord& record) {
    return false; // Not implemented
}

bool HDF5Writer::finalize() {
    return false; // Not implemented
}

} // namespace IgnisYeet::Output
