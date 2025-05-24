# Lavender - Automated Plant Watering System

Lavender is an automated plant watering system powered by an ESP32 microcontroller and developed in Rust. It monitors soil moisture levels and activates a pump to water plants when needed, displaying real-time status on an LCD screen. This project is designed to be a helpful tool for plant enthusiasts, ensuring plants receive adequate moisture with minimal manual intervention.

## Features

*   **Automated Watering:** Waters plants automatically when soil moisture drops below a set threshold.
*   **Real-time Monitoring:** Displays current soil moisture level and pump status on a 16x2 LCD screen.
*   **Last Watered Time:** Shows the time elapsed since the pump was last activated.
*   **Status LED:** Provides a visual indication of the system's operational status.
*   **Configurable Pins:** Easily configure ESP32 pins for sensors and actuators directly in `src/main.rs`.
*   **Wokwi Simulation:** Supports simulation in the Wokwi online ESP32 simulator.
*   **Rust Powered:** Developed using Rust for robust and efficient performance on embedded systems.

## Hardware and Software Requirements

### Hardware

*   **ESP32 Development Board:** Any ESP32 board (e.g., ESP32-DevKitC).
*   **Soil Moisture Sensor:** Analog output type.
*   **Mini Water Pump:** 3-6V DC, controlled by a GPIO pin (likely via a relay or transistor driver circuit, which you'll need to provide).
*   **LCD1602 Display:** I2C interface (PCF8574T backpack recommended).
*   **LED:** Standard LED for status indication.
*   **Jumper Wires and Breadboard:** For connections.
*   **External Power Supply (Optional but Recommended):** For the pump and ESP32, especially if the pump draws significant current.

### Software

*   **Rust:** Version 1.77 or later (as specified in `Cargo.toml`).
*   **ESP-IDF:** The Espressif IoT Development Framework. Ensure the toolchain is installed and environment variables are configured (refer to ESP-IDF documentation).
*   **`espflash` Utility:** For flashing the firmware to the ESP32. Install with `cargo install espflash`.
*   **Wokwi Account (Optional):** For online simulation (wokwi.com).

## Getting Started

Follow these steps to get the Lavender Automated Plant Watering System up and running on your ESP32.

### Prerequisites

1.  **Install Rust:** If you don't have Rust installed, visit [rust-lang.org](https://www.rust-lang.org/tools/install) for instructions.
2.  **Install ESP-IDF:** Set up the ESP-IDF toolchain and its dependencies. Follow the official Espressif documentation for your operating system: [ESP-IDF Get Started Guide](https://docs.espressif.com/projects/esp-idf/en/latest/esp32/get-started/index.html). Ensure that the ESP-IDF environment is activated in your terminal session (e.g., by running `source path/to/esp-idf/export.sh`).
3.  **Install `espflash`:** This utility is used to flash the compiled firmware onto the ESP32. Install it using Cargo:
    ```bash
    cargo install espflash
    ```

### Cloning the Repository

Clone this repository to your local machine:

```bash
git clone https://github.com/your-username/lavender.git # Replace with the actual repository URL
cd lavender
```

### Building the Project

Compile the project using Cargo. This will build the firmware for the ESP32.

```bash
cargo build --release
```

### Flashing the Project

Flash the compiled firmware to your ESP32. Connect your ESP32 to your computer via USB. Replace `<SERIAL_PORT>` with the actual serial port your ESP32 is connected to (e.g., `/dev/ttyUSB0` on Linux or `COM3` on Windows).

```bash
espflash flash target/xtensa-esp32-espidf/release/lavender --monitor <SERIAL_PORT>
```
The `--monitor` flag will open a serial monitor after flashing, allowing you to see log output from the device.

### Configuration

*   **Pin Configuration:** Hardware pin assignments for the LED, LCD, moisture sensor, and pump are defined in `src/main.rs`. You can modify these if your hardware setup differs.
*   **ESP-IDF Settings:** Project-specific ESP-IDF settings can be reviewed or modified in the `sdkconfig.defaults` file. For more advanced configurations, you might use `cargo menuconfig` (part of `esp-idf-svc`).

## Project Structure

The project is organized as follows:

```
.
├── .cargo/               # Cargo configuration
├── .devcontainer/        # VSCode Dev Container configuration
├── .github/              # GitHub Actions CI configuration
├── src/                  # Source code
│   ├── lcd.rs            # LCD display driver and functions
│   ├── led.rs            # LED control logic
│   ├── main.rs           # Main application entry point and core logic
│   ├── moisture.rs       # Moisture sensor reading and calibration
│   └── pump.rs           # Pump control logic
├── .gitignore            # Specifies intentionally untracked files that Git should ignore
├── Cargo.toml            # Rust package manager configuration (dependencies, project info)
├── build.rs              # Build script (e.g., for embuild)
├── diagram.json          # Wokwi simulator diagram (hardware connections)
├── rust-toolchain.toml   # Specifies the Rust toolchain version
├── sdkconfig.defaults    # Default ESP-IDF SDK configuration
└── wokwi.toml            # Wokwi simulator project configuration
```

*   **`/src`**: Contains all the Rust source code for the project.
    *   `main.rs`: The main application logic, including initialization of peripherals and the primary control loop.
    *   `lcd.rs`: Module for interfacing with the LCD1602 display.
    *   `led.rs`: Module for controlling the status LED.
    *   `moisture.rs`: Module for reading and interpreting data from the soil moisture sensor.
    *   `pump.rs`: Module for controlling the water pump.
*   **`Cargo.toml`**: The manifest file for this Rust package. It contains metadata such as package name, version, authors, and dependencies.
*   **`wokwi.toml`**: Configuration file for the Wokwi online ESP32 simulator.
*   **`diagram.json`**: Defines the hardware components and their connections for the Wokwi simulation.
*   **`.devcontainer/`**: Contains configuration for using VS Code Dev Containers, allowing for a consistent and reproducible development environment.
*   **`sdkconfig.defaults`**: Provides default configuration values for the ESP-IDF SDK. These can be customized, for example, by using `cargo espflash menuconfig`.

## How it Works

The system operates in a continuous loop defined in `src/main.rs`:

1.  **Initialization:**
    *   The ESP32 peripherals (GPIO, I2C, ADC) are initialized.
    *   Drivers for the LED, LCD, pump, and moisture sensor are instantiated with their respective pin configurations.

2.  **Main Loop:**
    *   The status LED is turned on at the beginning of a cycle.
    *   The moisture sensor takes multiple readings, and an average value is calculated to determine the current `MoistureLevel` (e.g., Wet, Moist, Dry, VeryDry).
    *   The LCD is updated to display:
        *   The current moisture level (e.g., "VeryDry") and the raw sensor value.
        *   The time elapsed since the pump was last turned on.
    *   **Pump Control:** If the `MoistureLevel` is `VeryDry` (or a similarly configured dry threshold):
        *   The pump is turned on for a predefined duration (e.g., 1 second).
        *   The LCD is updated to reflect that the pump is active and resets its "last on" time.
        *   The pump is then turned off.
    *   The system pauses for a short duration (e.g., 1 second).
    *   The status LED is turned off.
    *   The system pauses again before starting a new cycle.

This cycle ensures that the plant's moisture is regularly checked and water is provided as needed. The specific thresholds for moisture levels and pump duration can be adjusted in the source code (`src/moisture.rs` and `src/main.rs`).

## Simulation with Wokwi

This project can be simulated using [Wokwi](https://wokwi.com/), an online ESP32 and Arduino simulator. This allows you to test the logic and interactions without needing physical hardware.

The repository includes:
*   `wokwi.toml`: Configures the project for Wokwi, specifying the firmware ELF file.
*   `diagram.json`: Defines the virtual hardware setup in Wokwi, including the ESP32, LCD, and connections.

### Running the Simulation

1.  **Build the project in debug mode:** Wokwi typically uses the debug ELF file.
    ```bash
    cargo build
    ```
    This will produce an ELF file at `target/xtensa-esp32-espidf/debug/lavender`.

2.  **Open Wokwi:** Go to [wokwi.com](https://wokwi.com/).
3.  **Start a new ESP32 project in Wokwi.**
4.  **Upload your files:**
    *   In the Wokwi project view, you'll see a file manager tab. Upload the `wokwi.toml` and `diagram.json` files from this repository.
    *   You will also need to upload the compiled firmware ELF file (`target/xtensa-esp32-espidf/debug/lavender`) to the Wokwi project. Wokwi will use the path specified in `wokwi.toml`. If Wokwi cannot find the ELF file, you might need to adjust paths or upload it manually to the root of the Wokwi project files.

Once the files are set up, Wokwi should automatically use the `diagram.json` to arrange the components and `wokwi.toml` to load the firmware. You can then run the simulation to see the LCD output and interact with potential inputs (though this project primarily relies on sensor input which Wokwi can simulate).

*Note: Simulation of specific analog sensor behavior might require adjustments in Wokwi or the code for realistic results.*