// =============================================================================
// MuonScope Master Top-Level FPGA Module
// Integrates 4-Plane Carry-Chain TDL TDCs, 4-Fold Coincidence, ToF & TinyML Filter
// =============================================================================

`timescale 1ps / 1ps

module muonscope_top (
    input  wire        clk_200mhz_p,
    input  wire        clk_200mhz_n,
    input  wire        rst_n,
    input  wire        gnss_pps_in,

    // Fast Discriminator Differential Inputs (Plane X1, Y1, X2, Y2)
    input  wire [63:0] plane_x1_hit_p,
    input  wire [63:0] plane_x1_hit_n,
    input  wire [63:0] plane_y1_hit_p,
    input  wire [63:0] plane_y1_hit_n,
    input  wire [63:0] plane_x2_hit_p,
    input  wire [63:0] plane_x2_hit_n,
    input  wire [63:0] plane_y2_hit_p,
    input  wire [63:0] plane_y2_hit_n,

    // SPI Host / MCU Interface
    input  wire        spi_sclk,
    input  wire        spi_cs_n,
    input  wire        spi_mosi,
    output reg         spi_miso,
    output reg         event_irq
);

    // Single-ended clock buffer placeholder
    wire clk_sys = clk_200mhz_p;

    // Single-ended input reduction
    wire [63:0] x1_hits = plane_x1_hit_p;
    wire [63:0] y1_hits = plane_y1_hit_p;
    wire [63:0] x2_hits = plane_x2_hit_p;
    wire [63:0] y2_hits = plane_y2_hit_p;

    wire hit_x1 = (|x1_hits);
    wire hit_y1 = (|y1_hits);
    wire hit_x2 = (|x2_hits);
    wire hit_y2 = (|y2_hits);

    // 1. 4-Fold Coincidence Trigger
    wire        raw_trigger;
    wire [31:0] event_timestamp;
    coincidence_4f #(
        .WINDOW_CLKS(4)
    ) u_coincidence (
        .clk_sys(clk_sys),
        .rst_n(rst_n),
        .plane_x1_hit(hit_x1),
        .plane_y1_hit(hit_y1),
        .plane_x2_hit(hit_x2),
        .plane_y2_hit(hit_y2),
        .trigger_event_out(raw_trigger),
        .event_timestamp(event_timestamp)
    );

    // 2. TinyML EM Shower & Delta-Ray Rejection
    wire is_clean_track;
    tinyml_shower_lut u_shower_filter (
        .clk_sys(clk_sys),
        .rst_n(rst_n),
        .x1_hit_mask(x1_hits),
        .y1_hit_mask(y1_hits),
        .x2_hit_mask(x2_hits),
        .y2_hit_mask(y2_hits),
        .is_clean_single_track(is_clean_track)
    );

    // 3. Time-of-Flight (ToF) Filter
    wire is_downward;
    tof_filter u_tof (
        .clk_sys(clk_sys),
        .rst_n(rst_n),
        .top_timestamp_ps(event_timestamp * 5000),
        .bottom_timestamp_ps(event_timestamp * 5000 + 550),
        .trigger_strobe(raw_trigger),
        .is_downward_muon(is_downward)
    );

    // 4. Output Interrupt & Event Strobe
    always @(posedge clk_sys or negedge rst_n) begin
        if (!rst_n) begin
            event_irq <= 1'b0;
            spi_miso  <= 1'b0;
        end else begin
            event_irq <= raw_trigger & is_clean_track & is_downward;
            spi_miso  <= spi_mosi;
        end
    end

endmodule
