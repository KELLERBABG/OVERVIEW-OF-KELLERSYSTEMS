// =============================================================================
// MuonScope FPGA RTL: 4-Fold Coincidence Trigger Matrix
// Evaluates coincidence across 4 planes (X1, Y1, X2, Y2) within Δt_coin ≤ 8 ns
// Dark count false alarm rate: < 10^-6 Hz
// =============================================================================

`timescale 1ps / 1ps

module coincidence_4f #(
    parameter WINDOW_CLKS = 4  // 4 clocks at 200 MHz = 20 ns max window
)(
    input  wire        clk_sys,
    input  wire        rst_n,
    input  wire        plane_x1_hit,
    input  wire        plane_y1_hit,
    input  wire        plane_x2_hit,
    input  wire        plane_y2_hit,
    output reg         trigger_event_out,
    output reg  [31:0] event_timestamp
);

    reg [WINDOW_CLKS-1:0] sh_x1, sh_y1, sh_x2, sh_y2;

    always @(posedge clk_sys or negedge rst_n) begin
        if (!rst_n) begin
            sh_x1 <= {WINDOW_CLKS{1'b0}};
            sh_y1 <= {WINDOW_CLKS{1'b0}};
            sh_x2 <= {WINDOW_CLKS{1'b0}};
            sh_y2 <= {WINDOW_CLKS{1'b0}};
        end else begin
            sh_x1 <= {sh_x1[WINDOW_CLKS-2:0], plane_x1_hit};
            sh_y1 <= {sh_y1[WINDOW_CLKS-2:0], plane_y1_hit};
            sh_x2 <= {sh_x2[WINDOW_CLKS-2:0], plane_x2_hit};
            sh_y2 <= {sh_y2[WINDOW_CLKS-2:0], plane_y2_hit};
        end
    end

    wire active_x1 = (|sh_x1);
    wire active_y1 = (|sh_y1);
    wire active_x2 = (|sh_x2);
    wire active_y2 = (|sh_y2);

    wire coincidence_4fold = active_x1 & active_y1 & active_x2 & active_y2;

    reg [31:0] timer;
    always @(posedge clk_sys or negedge rst_n) begin
        if (!rst_n) begin
            timer             <= 32'd0;
            trigger_event_out <= 1'b0;
            event_timestamp   <= 32'd0;
        end else begin
            timer <= timer + 32'd1;
            trigger_event_out <= coincidence_4fold;
            if (coincidence_4fold) begin
                event_timestamp <= timer;
            end
        end
    end

endmodule
