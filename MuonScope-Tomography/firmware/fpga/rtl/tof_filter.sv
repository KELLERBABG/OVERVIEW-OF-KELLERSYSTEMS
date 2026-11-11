// =============================================================================
// MuonScope FPGA RTL: Time-of-Flight (ToF) Downward Vector Discriminator
// Verifies downward arrival order (t_top < t_bottom) and rejects upward albedo.
// =============================================================================

`timescale 1ps / 1ps

module tof_filter (
    input  wire        clk_sys,
    input  wire        rst_n,
    input  wire [31:0] top_timestamp_ps,
    input  wire [31:0] bottom_timestamp_ps,
    input  wire        trigger_strobe,
    output reg         is_downward_muon
);

    reg [31:0] dt;

    always @(posedge clk_sys or negedge rst_n) begin
        if (!rst_n) begin
            is_downward_muon <= 1'b0;
            dt               <= 32'd0;
        end else if (trigger_strobe) begin
            if (bottom_timestamp_ps > top_timestamp_ps) begin
                dt = bottom_timestamp_ps - top_timestamp_ps;
                is_downward_muon <= (dt >= 32'd200) && (dt <= 32'd2500);
            end else begin
                is_downward_muon <= 1'b0;
            end
        end else begin
            is_downward_muon <= 1'b0;
        end
    end

endmodule
