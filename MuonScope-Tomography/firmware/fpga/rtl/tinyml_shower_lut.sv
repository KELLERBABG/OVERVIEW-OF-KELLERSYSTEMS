// =============================================================================
// MuonScope FPGA RTL: TinyML Shower & Delta-Ray Rejection Filter
// Filters out multi-hit electromagnetic showers and knock-on delta electrons in < 50 ns
// =============================================================================

`timescale 1ps / 1ps

module tinyml_shower_lut (
    input  wire        clk_sys,
    input  wire        rst_n,
    input  wire [63:0] x1_hit_mask,
    input  wire [63:0] y1_hit_mask,
    input  wire [63:0] x2_hit_mask,
    input  wire [63:0] y2_hit_mask,
    output reg         is_clean_single_track
);

    // Fast population count for hit density evaluation
    function [6:0] count_bits(input [63:0] mask);
        integer j;
        begin
            count_bits = 7'd0;
            for (j = 0; j < 64; j = j + 1) begin
                count_bits = count_bits + mask[j];
            end
        end
    endfunction

    wire [6:0] n_x1 = count_bits(x1_hit_mask);
    wire [6:0] n_y1 = count_bits(y1_hit_mask);
    wire [6:0] n_x2 = count_bits(x2_hit_mask);
    wire [6:0] n_y2 = count_bits(y2_hit_mask);

    always @(posedge clk_sys or negedge rst_n) begin
        if (!rst_n) begin
            is_clean_single_track <= 1'b0;
        end else begin
            // Single muon: 1 or 2 adjacent hits per plane (triangular charge sharing)
            // Multi-hit electromagnetic cascade: > 3 hits on any single plane
            is_clean_single_track <= (n_x1 >= 1 && n_x1 <= 2) &&
                                     (n_y1 >= 1 && n_y1 <= 2) &&
                                     (n_x2 >= 1 && n_x2 <= 2) &&
                                     (n_y2 >= 1 && n_y2 <= 2);
        end
    end

endmodule
