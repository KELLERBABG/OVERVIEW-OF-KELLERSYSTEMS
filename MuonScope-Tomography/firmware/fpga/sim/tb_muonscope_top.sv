// =============================================================================
// MuonScope Testbench: Silicon Top-Level Trigger & Filter Verification
// =============================================================================

`timescale 1ps / 1ps

module tb_muonscope_top;

    reg clk_200mhz_p = 0;
    reg clk_200mhz_n = 1;
    reg rst_n = 0;
    reg gnss_pps = 0;

    reg [63:0] x1_p = 0, y1_p = 0, x2_p = 0, y2_p = 0;
    wire spi_miso, event_irq;

    always #2500 clk_200mhz_p = ~clk_200mhz_p;
    always #2500 clk_200mhz_n = ~clk_200mhz_n;

    muonscope_top dut (
        .clk_200mhz_p(clk_200mhz_p),
        .clk_200mhz_n(clk_200mhz_n),
        .rst_n(rst_n),
        .gnss_pps_in(gnss_pps),
        .plane_x1_hit_p(x1_p),
        .plane_x1_hit_n(~x1_p),
        .plane_y1_hit_p(y1_p),
        .plane_y1_hit_n(~y1_p),
        .plane_x2_hit_p(x2_p),
        .plane_x2_hit_n(~x2_p),
        .plane_y2_hit_p(y2_p),
        .plane_y2_hit_n(~y2_p),
        .spi_sclk(1'b0),
        .spi_cs_n(1'b1),
        .spi_mosi(1'b0),
        .spi_miso(spi_miso),
        .event_irq(event_irq)
    );

    initial begin
        #10000;
        rst_n = 1;
        #10000;

        // Test 1: Legitimate 4-fold single muon track (Hit on strip 10 of each plane)
        x1_p[10] = 1;
        y1_p[10] = 1;
        x2_p[10] = 1;
        y2_p[10] = 1;
        #5000;
        x1_p = 0; y1_p = 0; x2_p = 0; y2_p = 0;
        #20000;

        // Test 2: EM Shower cascade (Multi-hit on 5 strips) -> Must be rejected by TinyML
        x1_p[10:15] = 6'b111111;
        y1_p[10:15] = 6'b111111;
        x2_p[10:15] = 6'b111111;
        y2_p[10:15] = 6'b111111;
        #5000;
        x1_p = 0; y1_p = 0; x2_p = 0; y2_p = 0;
        #20000;

        $finish;
    end

endmodule
