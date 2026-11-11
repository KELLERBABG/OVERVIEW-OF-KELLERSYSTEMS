# ==============================================================================
# MuonScope™ Artix-7 (XC7A50T-2CSG324I) Master Physical & Timing Constraints
# Target Tool: Xilinx Vivado 2024.1+
# ==============================================================================

# ------------------------------------------------------------------------------
# 1. Primary Clocks & Clock Management
# ------------------------------------------------------------------------------
create_clock -period 5.000 -name clk_sys [get_ports clk_200mhz_p]
set_input_jitter [get_clocks clk_sys] 0.050

# ------------------------------------------------------------------------------
# 2. Hand-Placed Carry-Chain Tapped Delay Line (TDL) TDC Physical Constraints
# Locks CARRY4 primitives to dedicated vertical SLICE columns to enforce
# uniform propagation delay (<75 ps/tap) and eliminate routing skew.
# ------------------------------------------------------------------------------

# TDL Channel 0 Placement (Plane X1, Strip 0)
set_property LOC SLICE_X24Y50 [get_cells u_tdc_bank/gen_tdc[0].u_tdl/gen_carry[0].u_carry4]
set_property BEL CARRY4       [get_cells u_tdc_bank/gen_tdc[0].u_tdl/gen_carry[0].u_carry4]

set_property LOC SLICE_X24Y51 [get_cells u_tdc_bank/gen_tdc[0].u_tdl/gen_carry[1].u_carry4]
set_property BEL CARRY4       [get_cells u_tdc_bank/gen_tdc[0].u_tdl/gen_carry[1].u_carry4]

set_property LOC SLICE_X24Y52 [get_cells u_tdc_bank/gen_tdc[0].u_tdl/gen_carry[2].u_carry4]
set_property BEL CARRY4       [get_cells u_tdc_bank/gen_tdc[0].u_tdl/gen_carry[2].u_carry4]

set_property LOC SLICE_X24Y53 [get_cells u_tdc_bank/gen_tdc[0].u_tdl/gen_carry[3].u_carry4]
set_property BEL CARRY4       [get_cells u_tdc_bank/gen_tdc[0].u_tdl/gen_carry[3].u_carry4]

# TDL Channel 1 Placement (Plane X1, Strip 1)
set_property LOC SLICE_X26Y50 [get_cells u_tdc_bank/gen_tdc[1].u_tdl/gen_carry[0].u_carry4]
set_property BEL CARRY4       [get_cells u_tdc_bank/gen_tdc[1].u_tdl/gen_carry[0].u_carry4]

set_property LOC SLICE_X26Y51 [get_cells u_tdc_bank/gen_tdc[1].u_tdl/gen_carry[1].u_carry4]
set_property BEL CARRY4       [get_cells u_tdc_bank/gen_tdc[1].u_tdl/gen_carry[1].u_carry4]

set_property LOC SLICE_X26Y52 [get_cells u_tdc_bank/gen_tdc[1].u_tdl/gen_carry[2].u_carry4]
set_property BEL CARRY4       [get_cells u_tdc_bank/gen_tdc[1].u_tdl/gen_carry[2].u_carry4]

set_property LOC SLICE_X26Y53 [get_cells u_tdc_bank/gen_tdc[1].u_tdl/gen_carry[3].u_carry4]
set_property BEL CARRY4       [get_cells u_tdc_bank/gen_tdc[1].u_tdl/gen_carry[3].u_carry4]

# Enforce strict zero-hold routing optimization for TDL capture registers
set_property DONT_TOUCH true [get_cells u_tdc_bank/gen_tdc[*].u_tdl/*]

# ------------------------------------------------------------------------------
# 3. I/O Pin Allocations & Electrical Standards
# ------------------------------------------------------------------------------

# System Clock 200 MHz Differential (Bank 34, VCCO = 2.5V)
set_property PACKAGE_PIN R4 [get_ports clk_200mhz_p]
set_property PACKAGE_PIN T4 [get_ports clk_200mhz_n]
set_property IOSTANDARD DIFF_SSTL15 [get_ports clk_200mhz_*]

# System Asynchronous Reset Active-Low (Bank 14, VCCO = 3.3V)
set_property PACKAGE_PIN E16 [get_ports rst_n]
set_property IOSTANDARD LVCMOS33 [get_ports rst_n]

# Fast Discriminator Inputs (Plane X1, Strips 0..3) - Bank 35, LVDS_25
set_property PACKAGE_PIN H14 [get_ports {plane_x1_hit_p[0]}]
set_property PACKAGE_PIN G14 [get_ports {plane_x1_hit_n[0]}]
set_property PACKAGE_PIN H16 [get_ports {plane_x1_hit_p[1]}]
set_property PACKAGE_PIN G16 [get_ports {plane_x1_hit_n[1]}]
set_property PACKAGE_PIN J15 [get_ports {plane_x1_hit_p[2]}]
set_property PACKAGE_PIN H15 [get_ports {plane_x1_hit_n[2]}]
set_property PACKAGE_PIN K15 [get_ports {plane_x1_hit_p[3]}]
set_property PACKAGE_PIN J14 [get_ports {plane_x1_hit_n[3]}]
set_property IOSTANDARD LVDS_25 [get_ports {plane_x1_hit_*}]
set_property DIFF_TERM TRUE     [get_ports {plane_x1_hit_*}]

# SPI Interface to Host MCU / Microcontroller (Bank 14)
set_property PACKAGE_PIN C15 [get_ports spi_sclk]
set_property PACKAGE_PIN D15 [get_ports spi_mosi]
set_property PACKAGE_PIN E15 [get_ports spi_miso]
set_property PACKAGE_PIN F15 [get_ports spi_cs_n]
set_property IOSTANDARD LVCMOS33 [get_ports spi_*]

# GNSS PPS Time-Synchronization Input (Sub-Nanosecond Trigger)
set_property PACKAGE_PIN B16 [get_ports gnss_pps_in]
set_property IOSTANDARD LVCMOS33 [get_ports gnss_pps_in]

# ------------------------------------------------------------------------------
# 4. Timing Exceptions & False Paths
# ------------------------------------------------------------------------------
set_false_path -from [get_ports rst_n]
set_false_path -to [get_pins -hierarchical *sync_reg*/D]
set_false_path -from [get_cells u_tdc_bank/gen_tdc[*].u_tdl/gen_carry[*].u_carry4] -to [get_cells u_tdc_bank/gen_tdc[*].u_tdl/reg_sample_reg[*]]
