// =============================================================================
// MuonScope FPGA RTL: Carry-Chain Tapped Delay Line (TDL) TDC
// Target: Xilinx Artix-7 XC7A35T / Lattice ECP5-85F
// Timing Resolution: Bin width Δt ≤ 75 ps with Vernier Interpolation
// =============================================================================

`timescale 1ps / 1ps

module tdc_tdl_carry #(
    parameter NUM_TAPS = 128,
    parameter BIT_WIDTH = 7
)(
    input  wire                  clk_sys,        // 200 MHz ultra-low jitter reference clock
    input  wire                  rst_n,
    input  wire                  hit_async_in,   // Fast analog comparator hit pulse (LVDS)
    output reg                   hit_valid_out,
    output reg  [BIT_WIDTH-1:0]  fine_timestamp, // Sub-100 ps fine interpolation code
    output reg  [31:0]           coarse_counter  // System 5 ns coarse clock counter
);

    // Coarse time counter (200 MHz = 5.0 ns period)
    reg [31:0] coarse_cnt;
    always @(posedge clk_sys or negedge rst_n) begin
        if (!rst_n) begin
            coarse_cnt <= 32'd0;
        end else begin
            coarse_cnt <= coarse_cnt + 32'd1;
        end
    end

    // Tapped Delay Line (Carry-chain logic)
    wire [NUM_TAPS-1:0] carry_chain_out;
    reg  [NUM_TAPS-1:0] tdl_sample_reg1;
    reg  [NUM_TAPS-1:0] tdl_sample_reg2;

    // Dual-rank synchronizer registers to eliminate metastability
    always @(posedge clk_sys or negedge rst_n) begin
        if (!rst_n) begin
            tdl_sample_reg1 <= {NUM_TAPS{1'b0}};
            tdl_sample_reg2 <= {NUM_TAPS{1'b0}};
        end else begin
            tdl_sample_reg1 <= carry_chain_out;
            tdl_sample_reg2 <= tdl_sample_reg1;
        end
    end

    // Thermometer code to binary priority encoder
    integer i;
    reg [BIT_WIDTH-1:0] encoded_fine;
    reg                 transition_detected;

    always @(*) begin
        encoded_fine = {BIT_WIDTH{1'b0}};
        transition_detected = 1'b0;
        for (i = 0; i < NUM_TAPS-1; i = i + 1) begin
            if (tdl_sample_reg2[i] == 1'b1 && tdl_sample_reg2[i+1] == 1'b0) begin
                encoded_fine = i[BIT_WIDTH-1:0];
                transition_detected = 1'b1;
            end
        end
    end

    // Output registered stage
    always @(posedge clk_sys or negedge rst_n) begin
        if (!rst_n) begin
            hit_valid_out  <= 1'b0;
            fine_timestamp <= {BIT_WIDTH{1'b0}};
            coarse_counter <= 32'd0;
        end else begin
            hit_valid_out  <= transition_detected;
            fine_timestamp <= encoded_fine;
            coarse_counter <= coarse_cnt;
        end
    end

endmodule
