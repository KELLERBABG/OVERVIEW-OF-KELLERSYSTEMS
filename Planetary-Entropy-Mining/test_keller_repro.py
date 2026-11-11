# ============================================================================
#  Keller-Verify Generated pytest Reproduction Suite
#  Generated from SMT-LIB 2.6 formal verification counterexample models
# ============================================================================

import pytest


def test_keller_main_line_90():
    """Formal verification counterexample for main (line 90)
    Defect class: python-index-out-of-bounds
    Witness: [ap=0, args=0, artifact=0, codes=0, len_artifact_mac=0, rep=0, src=0, truth=0, win=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # main(ap=0, args=0, artifact=0, codes=0, len_artifact_mac=0, rep=0, src=0, truth=0, win=0)
        pass

def test_keller_main_line_96():
    """Formal verification counterexample for main (line 96)
    Defect class: python-index-out-of-bounds
    Witness: [ap=0, args=0, artifact=0, codes=0, len_nul=0, node=0, nul=0, rep=0, src=0, truth=0, win=0, window=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # main(ap=0, args=0, artifact=0, codes=0, len_nul=0, node=0, nul=0, rep=0, src=0, truth=0, win=0, window=0)
        pass

def test_keller_main_line_110():
    """Formal verification counterexample for main (line 110)
    Defect class: python-index-out-of-bounds
    Witness: [ap=0, args=0, artifact=0, capture=0, cert=0, codes=0, len_cert_mac_b64=0, node=0, nul=0, ok=0, rep=0, src=0, truth=0, win=0, window=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # main(ap=0, args=0, artifact=0, capture=0, cert=0, codes=0, len_cert_mac_b64=0, node=0, nul=0, ok=0, rep=0, src=0, truth=0, win=0, window=0)
        pass

def test_keller_main_line_72():
    """Formal verification counterexample for main (line 72)
    Defect class: python-index-out-of-bounds
    Witness: [StageResult=-1, ap=0, args=0, len_list=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # main(StageResult=-1, ap=0, args=0, len_list=0)
        pass

def test_keller_main_line_59():
    """Formal verification counterexample for main (line 59)
    Defect class: python-zero-division
    Witness: [ap=0, args=0, docs=0, lines=0, out=0, report=0, sources=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(ZeroDivisionError):
        # Call target under witness parameters
        # main(ap=0, args=0, docs=0, lines=0, out=0, report=0, sources=0)
        pass

def test_keller_build_input_line_41():
    """Formal verification counterexample for build_input (line 41)
    Defect class: python-zero-division
    Witness: [src=0, src_sample_rate=0, win=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(ZeroDivisionError):
        # Call target under witness parameters
        # build_input(src=0, src_sample_rate=0, win=0)
        pass

def test_keller_main_line_77():
    """Formal verification counterexample for main (line 77)
    Defect class: python-index-out-of-bounds
    Witness: [ap=0, args=0, data=0, len_data=0, out=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # main(ap=0, args=0, data=0, len_data=0, out=0)
        pass

def test_keller_main_line_77():
    """Formal verification counterexample for main (line 77)
    Defect class: python-index-out-of-bounds
    Witness: [ap=0, args=0, data=0, len_data=0, out=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # main(ap=0, args=0, data=0, len_data=0, out=0)
        pass

def test_keller_Settlement_settle_line_106():
    """Formal verification counterexample for Settlement.settle (line 106)
    Defect class: python-index-out-of-bounds
    Witness: [len_self_balances=0, self_payee=-1]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # Settlement.settle(len_self_balances=0, self_payee=-1)
        pass

def test_keller_Settlement_settle_line_109():
    """Formal verification counterexample for Settlement.settle (line 109)
    Defect class: python-index-out-of-bounds
    Witness: [len_self_balances=0, self_price=0, sender=-1, surplus=1, value=1]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # Settlement.settle(len_self_balances=0, self_price=0, sender=-1, surplus=1, value=1)
        pass

def test_keller_shannon_entropy_bytes_line_66():
    """Formal verification counterexample for shannon_entropy_bytes (line 66)
    Defect class: python-zero-division
    Witness: [arr=0, arr_size=0, counts=0, p=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(ZeroDivisionError):
        # Call target under witness parameters
        # shannon_entropy_bytes(arr=0, arr_size=0, counts=0, p=0)
        pass

def test_keller_shannon_entropy_bytes_line_67():
    """Formal verification counterexample for shannon_entropy_bytes (line 67)
    Defect class: python-index-out-of-bounds
    Witness: [arr=0, arr_size=0, counts=0, div0=(x!1, len_p=0, mod0=(x!1, nz=1, p=1, p_=1]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # shannon_entropy_bytes(arr=0, arr_size=0, counts=0, div0=(x!1, len_p=0, mod0=(x!1, nz=1, p=1, p_=1)
        pass

def test_keller_min_entropy_histogram_bits_line_78():
    """Formal verification counterexample for min_entropy_histogram_bits (line 78)
    Defect class: python-zero-division
    Witness: [arr=0, arr_size=0, counts=0, p_max=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(ZeroDivisionError):
        # Call target under witness parameters
        # min_entropy_histogram_bits(arr=0, arr_size=0, counts=0, p_max=0)
        pass

def test_keller_min_entropy_histogram_bits_line_80():
    """Formal verification counterexample for min_entropy_histogram_bits (line 80)
    Defect class: python-zero-division
    Witness: [arr=0, arr_size=0, bits=0, counts=0, h=0, p_max=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(ZeroDivisionError):
        # Call target under witness parameters
        # min_entropy_histogram_bits(arr=0, arr_size=0, bits=0, counts=0, h=0, p_max=0)
        pass

def test_keller__periodogram_line_105():
    """Formal verification counterexample for _periodogram (line 105)
    Defect class: python-zero-division
    Witness: [n_segs=1, nfft=0, samples_size=0, window=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(ZeroDivisionError):
        # Call target under witness parameters
        # _periodogram(n_segs=1, nfft=0, samples_size=0, window=0)
        pass

def test_keller__periodogram_line_106():
    """Formal verification counterexample for _periodogram (line 106)
    Defect class: python-index-out-of-bounds
    Witness: [len_samples=0, n_segs=1, nfft=0, samples_size=0, seg=0, window=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # _periodogram(len_samples=0, n_segs=1, nfft=0, samples_size=0, seg=0, window=0)
        pass

def test_keller__periodogram_line_110():
    """Formal verification counterexample for _periodogram (line 110)
    Defect class: python-zero-division
    Witness: [fft=0, freqs=0, n_segs=1, nfft=0, power=0, sample_rate=0.0, samples_size=0, seg=0, window=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(ZeroDivisionError):
        # Call target under witness parameters
        # _periodogram(fft=0, freqs=0, n_segs=1, nfft=0, power=0, sample_rate=0.0, samples_size=0, seg=0, window=0)
        pass

def test_keller_spectral_analysis_line_125():
    """Formal verification counterexample for spectral_analysis (line 125)
    Defect class: python-index-out-of-bounds
    Witness: [above=0, arr=0, band=0, db=0, floor=0, len_freqs=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # spectral_analysis(above=0, arr=0, band=0, db=0, floor=0, len_freqs=0)
        pass

def test_keller_spectral_analysis_line_125():
    """Formal verification counterexample for spectral_analysis (line 125)
    Defect class: python-index-out-of-bounds
    Witness: [above=0, arr=0, band=0, db=0, floor=0, len_freqs=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # spectral_analysis(above=0, arr=0, band=0, db=0, floor=0, len_freqs=0)
        pass

def test_keller_spectral_analysis_line_130():
    """Formal verification counterexample for spectral_analysis (line 130)
    Defect class: python-index-out-of-bounds
    Witness: [above=0, arr=0, band=0, cdf=0, db=0, floor=0, len_cdf=0, pw=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # spectral_analysis(above=0, arr=0, band=0, cdf=0, db=0, floor=0, len_cdf=0, pw=0)
        pass

def test_keller_spectral_analysis_line_131():
    """Formal verification counterexample for spectral_analysis (line 131)
    Defect class: python-index-out-of-bounds
    Witness: [above=0, arr=0, band=0, cdf=0, db=0, f_lo=0, floor=0, len_freqs=0, pw=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # spectral_analysis(above=0, arr=0, band=0, cdf=0, db=0, f_lo=0, floor=0, len_freqs=0, pw=0)
        pass

def test_keller_spectral_analysis_line_132():
    """Formal verification counterexample for spectral_analysis (line 132)
    Defect class: python-index-out-of-bounds
    Witness: [above=0, arr=0, band=0, cdf=0, db=0, f_hi=0, f_lo=0, floor=0, len_freqs=0, pw=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # spectral_analysis(above=0, arr=0, band=0, cdf=0, db=0, f_hi=0, f_lo=0, floor=0, len_freqs=0, pw=0)
        pass

def test_keller_spectral_analysis_line_135():
    """Formal verification counterexample for spectral_analysis (line 135)
    Defect class: python-index-out-of-bounds
    Witness: [above=0, arr=0, band=0, cdf=0, db=0, excess=0, f_hi=0, f_lo=0, floor=0, len_db=0, occupied=0, peak_idx=0, pw=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # spectral_analysis(above=0, arr=0, band=0, cdf=0, db=0, excess=0, f_hi=0, f_lo=0, floor=0, len_db=0, occupied=0, peak_idx=0, pw=0)
        pass

def test_keller_spectral_analysis_line_136():
    """Formal verification counterexample for spectral_analysis (line 136)
    Defect class: python-index-out-of-bounds
    Witness: [above=0, arr=0, band=0, cdf=0, db=0, excess=0, f_hi=0, f_lo=0, floor=0, len_freqs=0, occupied=0, peak=0, peak_idx=0, pw=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # spectral_analysis(above=0, arr=0, band=0, cdf=0, db=0, excess=0, f_hi=0, f_lo=0, floor=0, len_freqs=0, occupied=0, peak=0, peak_idx=0, pw=0)
        pass

def test_keller_entropy_report_line_166():
    """Formal verification counterexample for entropy_report (line 166)
    Defect class: python-index-out-of-bounds
    Witness: [arr=0, len_res=0, min_hist_byte=0, nist_bit=0, res=0, res_0=0, run_nist90b=true, shannon_byte=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # entropy_report(arr=0, len_res=0, min_hist_byte=0, nist_bit=0, res=0, res_0=0, run_nist90b=true, shannon_byte=0)
        pass

def test_keller_entropy_report_line_178():
    """Formal verification counterexample for entropy_report (line 178)
    Defect class: python-zero-division
    Witness: [arr=0, bits=0, conservative=0, min_hist_byte=0, per_byte=0, shannon_byte=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(ZeroDivisionError):
        # Call target under witness parameters
        # entropy_report(arr=0, bits=0, conservative=0, min_hist_byte=0, per_byte=0, shannon_byte=0)
        pass

def test_keller_RFNoiseSource_read_line_108():
    """Formal verification counterexample for RFNoiseSource.read (line 108)
    Defect class: python-zero-division
    Witness: [n=1, nyq=0, self_sample_rate=0, sos=0, white=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(ZeroDivisionError):
        # Call target under witness parameters
        # RFNoiseSource.read(n=1, nyq=0, self_sample_rate=0, sos=0, white=0)
        pass

def test_keller_RFNoiseSource_read_line_110():
    """Formal verification counterexample for RFNoiseSource.read (line 110)
    Defect class: python-zero-division
    Witness: [band=0, n=1, nyq=0, self_sample_rate=0, sos=0, white=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(ZeroDivisionError):
        # Call target under witness parameters
        # RFNoiseSource.read(band=0, n=1, nyq=0, self_sample_rate=0, sos=0, white=0)
        pass

def test_keller_RFNoiseSource_read_line_118():
    """Formal verification counterexample for RFNoiseSource.read (line 118)
    Defect class: python-index-out-of-bounds
    Witness: [band=0, burst=0, envelope=0, len_envelope=0, length=0, n=1, n_bursts=0, nyq=0, self_sample_rate=0, sos=0, start=0, white=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # RFNoiseSource.read(band=0, burst=0, envelope=0, len_envelope=0, length=0, n=1, n_bursts=0, nyq=0, self_sample_rate=0, sos=0, start=0, white=0)
        pass

def test_keller_SeismicNoiseSource_read_line_153():
    """Formal verification counterexample for SeismicNoiseSource.read (line 153)
    Defect class: python-zero-division
    Witness: [freqs=0, n=1, self_sample_rate=0, white=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(ZeroDivisionError):
        # Call target under witness parameters
        # SeismicNoiseSource.read(freqs=0, n=1, self_sample_rate=0, white=0)
        pass

def test_keller_SeismicNoiseSource_read_line_154():
    """Formal verification counterexample for SeismicNoiseSource.read (line 154)
    Defect class: python-index-out-of-bounds
    Witness: [freqs=0, len_freqs=0, n=1, white=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # SeismicNoiseSource.read(freqs=0, len_freqs=0, n=1, white=0)
        pass

def test_keller_SeismicNoiseSource_read_line_164():
    """Formal verification counterexample for SeismicNoiseSource.read (line 164)
    Defect class: python-index-out-of-bounds
    Witness: [amp=0, colored=0, freqs=0, i=0, len_colored=0, n=1, n_events=0, shaping=0, spec=0, white=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # SeismicNoiseSource.read(amp=0, colored=0, freqs=0, i=0, len_colored=0, n=1, n_events=0, shaping=0, spec=0, white=0)
        pass

def test_keller__pfunc_line_65():
    """Formal verification counterexample for _pfunc (line 65)
    Defect class: python-zero-division
    Witness: [p=0.0, q=1, r=-1, result=0, x=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(ZeroDivisionError):
        # Call target under witness parameters
        # _pfunc(p=0.0, q=1, r=-1, result=0, x=0)
        pass

def test_keller__prob_to_min_entropy_line_104():
    """Formal verification counterexample for _prob_to_min_entropy (line 104)
    Defect class: python-zero-division
    Witness: [bits_per_symbol=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(ZeroDivisionError):
        # Call target under witness parameters
        # _prob_to_min_entropy(bits_per_symbol=0)
        pass

def test_keller__prediction_pu_line_122():
    """Formal verification counterexample for _prediction_pu (line 122)
    Defect class: python-zero-division
    Witness: [n_pred=0, p0_fallback=1.0, p_global=0.0, p_prime=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(ZeroDivisionError):
        # Call target under witness parameters
        # _prediction_pu(n_pred=0, p0_fallback=1.0, p_global=0.0, p_prime=0)
        pass

def test_keller__prediction_pu_line_126():
    """Formal verification counterexample for _prediction_pu (line 126)
    Defect class: python-zero-division
    Witness: [bits_per_symbol=-2, p_local=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(ZeroDivisionError):
        # Call target under witness parameters
        # _prediction_pu(bits_per_symbol=-2, p_local=0)
        pass

def test_keller_mcv_line_140():
    """Formal verification counterexample for mcv (line 140)
    Defect class: python-zero-division
    Witness: [counts=0, n=0, p_hat=0, symbols_size=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(ZeroDivisionError):
        # Call target under witness parameters
        # mcv(counts=0, n=0, p_hat=0, symbols_size=0)
        pass

def test_keller_mcv_line_141():
    """Formal verification counterexample for mcv (line 141)
    Defect class: python-zero-division
    Witness: [counts=0, n=1, p_hat=0, pu=0, symbols_size=1]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(ZeroDivisionError):
        # Call target under witness parameters
        # mcv(counts=0, n=1, p_hat=0, pu=0, symbols_size=1)
        pass

def test_keller_collision_line_150():
    """Formal verification counterexample for collision (line 150)
    Defect class: python-index-out-of-bounds
    Witness: [bits_size=0, int=-1, len_list=0, n=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # collision(bits_size=0, int=-1, len_list=0, n=0)
        pass

def test_keller_collision_line_160():
    """Formal verification counterexample for collision (line 160)
    Defect class: python-zero-division
    Witness: [bits_size=0, i=1, len_t_vals=0, n=0, v=0, x_bar=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(ZeroDivisionError):
        # Call target under witness parameters
        # collision(bits_size=0, i=1, len_t_vals=0, n=0, v=0, x_bar=0)
        pass

def test_keller_collision_line_162():
    """Formal verification counterexample for collision (line 162)
    Defect class: python-zero-division
    Witness: [bits_size=0, i=1, len_t_vals=1, n=0, sigma_hat=0, sq=0, v=1, x_bar=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(ZeroDivisionError):
        # Call target under witness parameters
        # collision(bits_size=0, i=1, len_t_vals=1, n=0, sigma_hat=0, sq=0, v=1, x_bar=0)
        pass

def test_keller_collision_line_163():
    """Formal verification counterexample for collision (line 163)
    Defect class: python-zero-division
    Witness: [bits_size=0, div0=(x!1, i=1, len_t_vals=0, mod0=(x!1, n=0, sigma_hat=0, sq=0, v=0, x_bar=0, x_bar_prime=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(ZeroDivisionError):
        # Call target under witness parameters
        # collision(bits_size=0, div0=(x!1, i=1, len_t_vals=0, mod0=(x!1, n=0, sigma_hat=0, sq=0, v=0, x_bar=0, x_bar_prime=0)
        pass

def test_keller_markov_line_189():
    """Formal verification counterexample for markov (line 189)
    Defect class: python-zero-division
    Witness: [b=0, bits_size=0, n=0, p0=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(ZeroDivisionError):
        # Call target under witness parameters
        # markov(b=0, bits_size=0, n=0, p0=0)
        pass

def test_keller_markov_line_191():
    """Formal verification counterexample for markov (line 191)
    Defect class: python-index-out-of-bounds
    Witness: [b=0, bits_size=0, c00=0, len_b=0, n=0, p0=0, p1=1]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # markov(b=0, bits_size=0, c00=0, len_b=0, n=0, p0=0, p1=1)
        pass

def test_keller_markov_line_191():
    """Formal verification counterexample for markov (line 191)
    Defect class: python-index-out-of-bounds
    Witness: [b=0, bits_size=0, c00=0, len_b=0, n=0, p0=0, p1=1]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # markov(b=0, bits_size=0, c00=0, len_b=0, n=0, p0=0, p1=1)
        pass

def test_keller_markov_line_192():
    """Formal verification counterexample for markov (line 192)
    Defect class: python-index-out-of-bounds
    Witness: [b=0, bits_size=0, c00=0, c01=0, len_b=0, n=0, p0=0, p1=1]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # markov(b=0, bits_size=0, c00=0, c01=0, len_b=0, n=0, p0=0, p1=1)
        pass

def test_keller_markov_line_192():
    """Formal verification counterexample for markov (line 192)
    Defect class: python-index-out-of-bounds
    Witness: [b=0, bits_size=0, c00=0, c01=0, len_b=0, n=0, p0=0, p1=1]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # markov(b=0, bits_size=0, c00=0, c01=0, len_b=0, n=0, p0=0, p1=1)
        pass

def test_keller_markov_line_193():
    """Formal verification counterexample for markov (line 193)
    Defect class: python-index-out-of-bounds
    Witness: [b=0, bits_size=0, c00=0, c01=0, c10=0, len_b=0, n=0, p0=0, p1=1]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # markov(b=0, bits_size=0, c00=0, c01=0, c10=0, len_b=0, n=0, p0=0, p1=1)
        pass

def test_keller_markov_line_193():
    """Formal verification counterexample for markov (line 193)
    Defect class: python-index-out-of-bounds
    Witness: [b=0, bits_size=0, c00=0, c01=0, c10=0, len_b=0, n=0, p0=0, p1=1]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # markov(b=0, bits_size=0, c00=0, c01=0, c10=0, len_b=0, n=0, p0=0, p1=1)
        pass

def test_keller_markov_line_194():
    """Formal verification counterexample for markov (line 194)
    Defect class: python-index-out-of-bounds
    Witness: [b=0, bits_size=0, c00=0, c01=0, c10=0, c11=0, len_b=0, n=0, p0=0, p1=1]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # markov(b=0, bits_size=0, c00=0, c01=0, c10=0, c11=0, len_b=0, n=0, p0=0, p1=1)
        pass

def test_keller_markov_line_194():
    """Formal verification counterexample for markov (line 194)
    Defect class: python-index-out-of-bounds
    Witness: [b=0, bits_size=0, c00=0, c01=0, c10=0, c11=0, len_b=0, n=0, p0=0, p1=1]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # markov(b=0, bits_size=0, c00=0, c01=0, c10=0, c11=0, len_b=0, n=0, p0=0, p1=1)
        pass

def test_keller_markov_line_197():
    """Formal verification counterexample for markov (line 197)
    Defect class: python-zero-division
    Witness: [b=0, bits_size=0, c00=0, c01=0, c10=0, c11=0, n=0, p0=0, p00=0, p1=1, s0=0, s1=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(ZeroDivisionError):
        # Call target under witness parameters
        # markov(b=0, bits_size=0, c00=0, c01=0, c10=0, c11=0, n=0, p0=0, p00=0, p1=1, s0=0, s1=0)
        pass

def test_keller_markov_line_198():
    """Formal verification counterexample for markov (line 198)
    Defect class: python-zero-division
    Witness: [b=0, bits_size=0, c00=0, c01=0, c10=0, c11=0, n=0, p0=0, p00=0, p01=0, p1=1, s0=0, s1=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(ZeroDivisionError):
        # Call target under witness parameters
        # markov(b=0, bits_size=0, c00=0, c01=0, c10=0, c11=0, n=0, p0=0, p00=0, p01=0, p1=1, s0=0, s1=0)
        pass

def test_keller_markov_line_199():
    """Formal verification counterexample for markov (line 199)
    Defect class: python-zero-division
    Witness: [b=0, bits_size=0, c00=0, c01=0, c10=0, c11=0, n=0, p0=0, p00=0, p01=0, p1=1, p10=0, s0=0, s1=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(ZeroDivisionError):
        # Call target under witness parameters
        # markov(b=0, bits_size=0, c00=0, c01=0, c10=0, c11=0, n=0, p0=0, p00=0, p01=0, p1=1, p10=0, s0=0, s1=0)
        pass

def test_keller_markov_line_200():
    """Formal verification counterexample for markov (line 200)
    Defect class: python-zero-division
    Witness: [b=0, bits_size=0, c00=0, c01=0, c10=0, c11=0, n=0, p0=0, p00=0, p01=0, p1=1, p10=0, p11=0, s0=0, s1=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(ZeroDivisionError):
        # Call target under witness parameters
        # markov(b=0, bits_size=0, c00=0, c01=0, c10=0, c11=0, n=0, p0=0, p00=0, p01=0, p1=1, p10=0, p11=0, s0=0, s1=0)
        pass

def test_keller_compression_line_222():
    """Formal verification counterexample for compression (line 222)
    Defect class: python-index-out-of-bounds
    Witness: [b=6, bits_size=0, blocks=0, len_bits=0, n=0, s_prime=2]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # compression(b=6, bits_size=0, blocks=0, len_bits=0, n=0, s_prime=2)
        pass

def test_keller__max_tuple_count_line_289():
    """Formal verification counterexample for _max_tuple_count (line 289)
    Defect class: python-index-out-of-bounds
    Witness: [b=62, bits_per_symbol=62, len_dict=0, mask=62, n=0, symbols_size=0, t=1]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # _max_tuple_count(b=62, bits_per_symbol=62, len_dict=0, mask=62, n=0, symbols_size=0, t=1)
        pass

def test_keller__max_tuple_count_line_292():
    """Formal verification counterexample for _max_tuple_count (line 292)
    Defect class: python-index-out-of-bounds
    Witness: [b=-40, bits_per_symbol=-40, i=62, key=0, len_symbols=0, mask=40, n=0, symbols_size=0, t=-1]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # _max_tuple_count(b=-40, bits_per_symbol=-40, i=62, key=0, len_symbols=0, mask=40, n=0, symbols_size=0, t=-1)
        pass

def test_keller__max_tuple_count_line_294():
    """Formal verification counterexample for _max_tuple_count (line 294)
    Defect class: python-index-out-of-bounds
    Witness: [b=51, bits_per_symbol=51, i=-2, key=0, len_counts=0, mask=-51, n=0, symbols_size=0, t=-1]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # _max_tuple_count(b=51, bits_per_symbol=51, i=-2, key=0, len_counts=0, mask=-51, n=0, symbols_size=0, t=-1)
        pass

def test_keller__max_tuple_count_line_299():
    """Formal verification counterexample for _max_tuple_count (line 299)
    Defect class: python-index-out-of-bounds
    Witness: [b=0, bits_per_symbol=0, counts=0, key=0, len_raw=0, n=0, raw=0, raw_0=0, symbols_size=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # _max_tuple_count(b=0, bits_per_symbol=0, counts=0, key=0, len_raw=0, n=0, raw=0, raw_0=0, symbols_size=0)
        pass

def test_keller__max_tuple_count_line_300():
    """Formal verification counterexample for _max_tuple_count (line 300)
    Defect class: python-index-out-of-bounds
    Witness: [b=0, bits_per_symbol=0, counts=0, key=-1, len_counts=0, n=0, raw=0, raw_0=-1, symbols_size=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # _max_tuple_count(b=0, bits_per_symbol=0, counts=0, key=-1, len_counts=0, n=0, raw=0, raw_0=-1, symbols_size=0)
        pass

def test_keller_t_tuple_line_321():
    """Formal verification counterexample for t_tuple (line 321)
    Defect class: python-zero-division
    Witness: [i=0, n=-1, p_max=0, pi=0, qi=0, symbols_size=-1, t_max=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(ZeroDivisionError):
        # Call target under witness parameters
        # t_tuple(i=0, n=-1, p_max=0, pi=0, qi=0, symbols_size=-1, t_max=0)
        pass

def test_keller_t_tuple_line_322():
    """Formal verification counterexample for t_tuple (line 322)
    Defect class: python-zero-division
    Witness: [div0=(x!1, i=0, mod0=(x!1, n=-1, p_max=0, pi=0, qi=0, symbols_size=-1, t_max=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(ZeroDivisionError):
        # Call target under witness parameters
        # t_tuple(div0=(x!1, i=0, mod0=(x!1, n=-1, p_max=0, pi=0, qi=0, symbols_size=-1, t_max=0)
        pass

def test_keller_lrs_line_348():
    """Formal verification counterexample for lrs (line 348)
    Defect class: python-index-out-of-bounds
    Witness: [b=0, bits_per_symbol=0, len_dict=0, n=0, p_max=0, symbols_size=0, u=1, v=1]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # lrs(b=0, bits_per_symbol=0, len_dict=0, n=0, p_max=0, symbols_size=0, u=1, v=1)
        pass

def test_keller_lrs_line_353():
    """Formal verification counterexample for lrs (line 353)
    Defect class: python-index-out-of-bounds
    Witness: [b=0, bits_per_symbol=0, i=6, key=0, len_symbols=0, mask=0, n=0, p_max=0, symbols_size=0, u=1, v=1, w=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # lrs(b=0, bits_per_symbol=0, i=6, key=0, len_symbols=0, mask=0, n=0, p_max=0, symbols_size=0, u=1, v=1, w=0)
        pass

def test_keller_lrs_line_355():
    """Formal verification counterexample for lrs (line 355)
    Defect class: python-index-out-of-bounds
    Witness: [b=0, bits_per_symbol=0, i=0, key=0, len_counts=0, mask=0, n=0, p_max=0, symbols_size=0, u=1, v=1, w=1]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # lrs(b=0, bits_per_symbol=0, i=0, key=0, len_counts=0, mask=0, n=0, p_max=0, symbols_size=0, u=1, v=1, w=1)
        pass

def test_keller_lrs_line_360():
    """Formal verification counterexample for lrs (line 360)
    Defect class: python-index-out-of-bounds
    Witness: [b=63, bits_per_symbol=63, counts=0, key=0, len_raw=0, mask=0, n=0, p_max=0, raw=0, raw_0=0, symbols_size=0, u=1, v=1, w=1]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # lrs(b=63, bits_per_symbol=63, counts=0, key=0, len_raw=0, mask=0, n=0, p_max=0, raw=0, raw_0=0, symbols_size=0, u=1, v=1, w=1)
        pass

def test_keller_lrs_line_361():
    """Formal verification counterexample for lrs (line 361)
    Defect class: python-index-out-of-bounds
    Witness: [b=63, bits_per_symbol=63, counts=0, key=6, len_counts=0, mask=0, n=0, p_max=0, raw=0, raw_0=6, symbols_size=0, u=1, v=1, w=1]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # lrs(b=63, bits_per_symbol=63, counts=0, key=6, len_counts=0, mask=0, n=0, p_max=0, raw=0, raw_0=6, symbols_size=0, u=1, v=1, w=1)
        pass

def test_keller_lrs_line_366():
    """Formal verification counterexample for lrs (line 366)
    Defect class: python-zero-division
    Witness: [b=0, bits_per_symbol=0, div0=(x!1, mask=0, mod0=(x!1, n=1, p_max=0, pairs=1, symbols_size=1, total=0, u=1, v=1, w=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(ZeroDivisionError):
        # Call target under witness parameters
        # lrs(b=0, bits_per_symbol=0, div0=(x!1, mask=0, mod0=(x!1, n=1, p_max=0, pairs=1, symbols_size=1, total=0, u=1, v=1, w=0)
        pass

def test_keller_lz78y_line_384():
    """Formal verification counterexample for lz78y (line 384)
    Defect class: python-index-out-of-bounds
    Witness: [b=0, len_dict=0, max_dict=65536, n=1, n_pred=0, s=0, symbols_size=1]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # lz78y(b=0, len_dict=0, max_dict=65536, n=1, n_pred=0, s=0, symbols_size=1)
        pass

def test_keller_lz78y_line_384():
    """Formal verification counterexample for lz78y (line 384)
    Defect class: python-index-out-of-bounds
    Witness: [b=0, len_dict=0, max_dict=65536, n=1, n_pred=0, s=0, symbols_size=1]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # lz78y(b=0, len_dict=0, max_dict=65536, n=1, n_pred=0, s=0, symbols_size=1)
        pass

def test_keller_lz78y_line_389():
    """Formal verification counterexample for lz78y (line 389)
    Defect class: python-index-out-of-bounds
    Witness: [b=0, correct=0, ctx=0, dict_size=0, len_s=0, max_dict=65536, n=1, n_pred=0, s=0, symbols_size=1]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # lz78y(b=0, correct=0, ctx=0, dict_size=0, len_s=0, max_dict=65536, n=1, n_pred=0, s=0, symbols_size=1)
        pass

def test_keller_lz78y_line_390():
    """Formal verification counterexample for lz78y (line 390)
    Defect class: python-index-out-of-bounds
    Witness: [b=0, correct=0, ctx=0, dict_size=0, i=0, len_s=0, max_dict=65536, n=1, n_pred=0, nxt=0, s=0, symbols_size=1]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # lz78y(b=0, correct=0, ctx=0, dict_size=0, i=0, len_s=0, max_dict=65536, n=1, n_pred=0, nxt=0, s=0, symbols_size=1)
        pass

def test_keller_lz78y_line_394():
    """Formal verification counterexample for lz78y (line 394)
    Defect class: python-index-out-of-bounds
    Witness: [b=0, correct=0, ctx=0, dict_size=0, len_d=0, max_dict=65536, n=1, n_pred=0, nxt=0, s=0, sub=0, symbols_size=1]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # lz78y(b=0, correct=0, ctx=0, dict_size=0, len_d=0, max_dict=65536, n=1, n_pred=0, nxt=0, s=0, sub=0, symbols_size=1)
        pass

def test_keller_lz78y_line_397():
    """Formal verification counterexample for lz78y (line 397)
    Defect class: python-index-out-of-bounds
    Witness: [b=0, correct=0, ctx=0, dict_size=0, len_sub=0, max_dict=65536, n=1, n_pred=0, nxt=0, s=0, sub=0, symbols_size=1]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # lz78y(b=0, correct=0, ctx=0, dict_size=0, len_sub=0, max_dict=65536, n=1, n_pred=0, nxt=0, s=0, sub=0, symbols_size=1)
        pass

def test_keller_lz78y_line_398():
    """Formal verification counterexample for lz78y (line 398)
    Defect class: python-index-out-of-bounds
    Witness: [b=0, correct=0, dict_size=0, int=-1, len_Optional=0, max_dict=65536, n=1, n_pred=0, s=0, symbols_size=1]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # lz78y(b=0, correct=0, dict_size=0, int=-1, len_Optional=0, max_dict=65536, n=1, n_pred=0, s=0, symbols_size=1)
        pass

def test_keller_lz78y_line_401():
    """Formal verification counterexample for lz78y (line 401)
    Defect class: python-index-out-of-bounds
    Witness: [b=0, best_count=0, correct=0, ctx=0, dict_size=0, len_s=0, max_dict=65536, n=1, n_pred=0, s=0, symbols_size=1]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # lz78y(b=0, best_count=0, correct=0, ctx=0, dict_size=0, len_s=0, max_dict=65536, n=1, n_pred=0, s=0, symbols_size=1)
        pass

def test_keller_lz78y_line_405():
    """Formal verification counterexample for lz78y (line 405)
    Defect class: python-index-out-of-bounds
    Witness: [b=0, best_count=0, correct=0, ctx=0, dict_size=0, k=-1, len_sub=0, max_dict=65536, n=1, n_pred=0, s=0, sub=0, symbols_size=1, y=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # lz78y(b=0, best_count=0, correct=0, ctx=0, dict_size=0, k=-1, len_sub=0, max_dict=65536, n=1, n_pred=0, s=0, sub=0, symbols_size=1, y=0)
        pass

def test_keller_lz78y_line_412():
    """Formal verification counterexample for lz78y (line 412)
    Defect class: python-zero-division
    Witness: [b=0, c=0, correct=0, dict_size=0, max_dict=65536, n=1, n_pred=0, p_global=0, s=0, symbols_size=1]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(ZeroDivisionError):
        # Call target under witness parameters
        # lz78y(b=0, c=0, correct=0, dict_size=0, max_dict=65536, n=1, n_pred=0, p_global=0, s=0, symbols_size=1)
        pass

def test_keller__SlidingMostFrequent___init___line_432():
    """Formal verification counterexample for _SlidingMostFrequent.__init__ (line 432)
    Defect class: python-index-out-of-bounds
    Witness: [int=-1, len_deque=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # _SlidingMostFrequent.__init__(int=-1, len_deque=0)
        pass

def test_keller__SlidingMostFrequent___init___line_433():
    """Formal verification counterexample for _SlidingMostFrequent.__init__ (line 433)
    Defect class: python-index-out-of-bounds
    Witness: [len_dict=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # _SlidingMostFrequent.__init__(len_dict=0)
        pass

def test_keller__SlidingMostFrequent___init___line_434():
    """Formal verification counterexample for _SlidingMostFrequent.__init__ (line 434)
    Defect class: python-index-out-of-bounds
    Witness: [len_dict=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # _SlidingMostFrequent.__init__(len_dict=0)
        pass

def test_keller__SlidingMostFrequent___init___line_435():
    """Formal verification counterexample for _SlidingMostFrequent.__init__ (line 435)
    Defect class: python-index-out-of-bounds
    Witness: [len_list=0, tuple_0=-1]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # _SlidingMostFrequent.__init__(len_list=0, tuple_0=-1)
        pass

def test_keller__SlidingMostFrequent___init___line_435():
    """Formal verification counterexample for _SlidingMostFrequent.__init__ (line 435)
    Defect class: python-index-out-of-bounds
    Witness: [len_tuple=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # _SlidingMostFrequent.__init__(len_tuple=0)
        pass

def test_keller__SlidingMostFrequent_push_line_447():
    """Formal verification counterexample for _SlidingMostFrequent.push (line 447)
    Defect class: python-index-out-of-bounds
    Witness: [len_self_counts=0, sym=-1]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # _SlidingMostFrequent.push(len_self_counts=0, sym=-1)
        pass

def test_keller__SlidingMostFrequent_push_line_448():
    """Formal verification counterexample for _SlidingMostFrequent.push (line 448)
    Defect class: python-index-out-of-bounds
    Witness: [len_self_last_seen=0, sym=-1]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # _SlidingMostFrequent.push(len_self_last_seen=0, sym=-1)
        pass

def test_keller__SlidingMostFrequent_push_line_449():
    """Formal verification counterexample for _SlidingMostFrequent.push (line 449)
    Defect class: python-index-out-of-bounds
    Witness: [len_self_counts=0, sym=-1]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # _SlidingMostFrequent.push(len_self_counts=0, sym=-1)
        pass

def test_keller_multi_mcw_line_471():
    """Formal verification counterexample for multi_mcw (line 471)
    Defect class: python-index-out-of-bounds
    Witness: [int=-1, len_list=0, max_win=2, n=0, symbols_size=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # multi_mcw(int=-1, len_list=0, max_win=2, n=0, symbols_size=0)
        pass

def test_keller_multi_mcw_line_481():
    """Formal verification counterexample for multi_mcw (line 481)
    Defect class: python-index-out-of-bounds
    Witness: [len_symbols=0, max_win=2, n=0, n_pred=-3, symbols_size=0, trackers=0, w=1]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # multi_mcw(len_symbols=0, max_win=2, n=0, n_pred=-3, symbols_size=0, trackers=0, w=1)
        pass

def test_keller_multi_mcw_line_487():
    """Formal verification counterexample for multi_mcw (line 487)
    Defect class: python-index-out-of-bounds
    Witness: [correct=0, len_mfs=0, len_wins=0, max_win=2, mfs=0, mfs_winner=0, n=0, n_pred=-3, prediction=0, scoreboard=0, symbols_size=0, trackers=0, w=1, winner=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # multi_mcw(correct=0, len_mfs=0, len_wins=0, max_win=2, mfs=0, mfs_winner=0, n=0, n_pred=-3, prediction=0, scoreboard=0, symbols_size=0, trackers=0, w=1, winner=0)
        pass

def test_keller_multi_mcw_line_494():
    """Formal verification counterexample for multi_mcw (line 494)
    Defect class: python-index-out-of-bounds
    Witness: [correct=0, j=-1, len_scoreboard=0, len_wins=0, max_win=2, mfs=0, mfs_winner=0, n=0, n_pred=-3, prediction=0, scoreboard=0, symbols_size=0, trackers=0, w=1, winner=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # multi_mcw(correct=0, j=-1, len_scoreboard=0, len_wins=0, max_win=2, mfs=0, mfs_winner=0, n=0, n_pred=-3, prediction=0, scoreboard=0, symbols_size=0, trackers=0, w=1, winner=0)
        pass

def test_keller_multi_mcw_line_498():
    """Formal verification counterexample for multi_mcw (line 498)
    Defect class: python-index-out-of-bounds
    Witness: [correct=0, i=-1, len_symbols=0, len_wins=0, max_win=2, mfs=0, mfs_winner=0, n=0, n_pred=-3, prediction=0, scoreboard=0, symbols_size=0, trackers=0, w=1, winner=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # multi_mcw(correct=0, i=-1, len_symbols=0, len_wins=0, max_win=2, mfs=0, mfs_winner=0, n=0, n_pred=-3, prediction=0, scoreboard=0, symbols_size=0, trackers=0, w=1, winner=0)
        pass

def test_keller_multi_mcw_line_500():
    """Formal verification counterexample for multi_mcw (line 500)
    Defect class: python-zero-division
    Witness: [c=0, correct=0, len_wins=0, max_win=2, n=3, n_pred=0, p_global=0, scoreboard=0, symbols_size=3, trackers=0, w=1, winner=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(ZeroDivisionError):
        # Call target under witness parameters
        # multi_mcw(c=0, correct=0, len_wins=0, max_win=2, n=3, n_pred=0, p_global=0, scoreboard=0, symbols_size=3, trackers=0, w=1, winner=0)
        pass

def test_keller_lag_prediction_line_522():
    """Formal verification counterexample for lag_prediction (line 522)
    Defect class: python-index-out-of-bounds
    Witness: [d=0, len_s=0, matches=1, n=1, packed=0, s=0, s_0=0, symbols_size=1]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # lag_prediction(d=0, len_s=0, matches=1, n=1, packed=0, s=0, s_0=0, symbols_size=1)
        pass

def test_keller_lag_prediction_line_522():
    """Formal verification counterexample for lag_prediction (line 522)
    Defect class: python-index-out-of-bounds
    Witness: [d=0, len_s=0, matches=1, n=1, packed=0, s=0, s_0=0, symbols_size=1]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # lag_prediction(d=0, len_s=0, matches=1, n=1, packed=0, s=0, s_0=0, symbols_size=1)
        pass

def test_keller_lag_prediction_line_524():
    """Formal verification counterexample for lag_prediction (line 524)
    Defect class: python-index-out-of-bounds
    Witness: [counts=0, counts_0=0, d=-8, k=0, len_counts=0, matches=1, n=8, packed=0, s=0, s_0=0, symbols_size=8, window=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # lag_prediction(counts=0, counts_0=0, d=-8, k=0, len_counts=0, matches=1, n=8, packed=0, s=0, s_0=0, symbols_size=8, window=0)
        pass

def test_keller_lag_prediction_line_525():
    """Formal verification counterexample for lag_prediction (line 525)
    Defect class: python-index-out-of-bounds
    Witness: [counts=0, counts_0=0, d=-8, k=0, len_packed=0, matches=1, n=8, packed=0, s=0, s_0=0, symbols_size=8, window=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # lag_prediction(counts=0, counts_0=0, d=-8, k=0, len_packed=0, matches=1, n=8, packed=0, s=0, s_0=0, symbols_size=8, window=0)
        pass

def test_keller_lag_prediction_line_525():
    """Formal verification counterexample for lag_prediction (line 525)
    Defect class: python-index-out-of-bounds
    Witness: [counts=0, counts_0=0, d=-8, k=0, len_packed=0, matches=1, n=8, packed=0, s=0, s_0=0, symbols_size=8, window=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # lag_prediction(counts=0, counts_0=0, d=-8, k=0, len_packed=0, matches=1, n=8, packed=0, s=0, s_0=0, symbols_size=8, window=0)
        pass

def test_keller_lag_prediction_line_526():
    """Formal verification counterexample for lag_prediction (line 526)
    Defect class: python-zero-division
    Witness: [d=-1, n=1, packed=0, s=0, symbols_size=1, winner=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(ZeroDivisionError):
        # Call target under witness parameters
        # lag_prediction(d=-1, n=1, packed=0, s=0, symbols_size=1, winner=0)
        pass

def test_keller_lag_prediction_line_526():
    """Formal verification counterexample for lag_prediction (line 526)
    Defect class: python-index-out-of-bounds
    Witness: [d=0, len_packed=0, n=1, packed=0, s=0, symbols_size=1, winner=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # lag_prediction(d=0, len_packed=0, n=1, packed=0, s=0, symbols_size=1, winner=0)
        pass

def test_keller_lag_prediction_line_530():
    """Formal verification counterexample for lag_prediction (line 530)
    Defect class: python-index-out-of-bounds
    Witness: [correct=0, d=0, idx=0, idx_valid=0, len_idx=0, n=1, packed=0, s=0, safe=0, symbols_size=1, valid=1, winner=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # lag_prediction(correct=0, d=0, idx=0, idx_valid=0, len_idx=0, n=1, packed=0, s=0, safe=0, symbols_size=1, valid=1, winner=0)
        pass

def test_keller_lag_prediction_line_531():
    """Formal verification counterexample for lag_prediction (line 531)
    Defect class: python-index-out-of-bounds
    Witness: [correct=0, d=0, idx=0, idx_valid=0, len_correct=0, n=1, packed=0, s=0, safe=0, symbols_size=1, valid=1, winner=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # lag_prediction(correct=0, d=0, idx=0, idx_valid=0, len_correct=0, n=1, packed=0, s=0, safe=0, symbols_size=1, valid=1, winner=0)
        pass

def test_keller_lag_prediction_line_531():
    """Formal verification counterexample for lag_prediction (line 531)
    Defect class: python-index-out-of-bounds
    Witness: [correct=0, d=0, idx=0, idx_valid=-1, len_s=0, n=2, packed=0, s=0, safe=-1, symbols_size=2, valid=1, winner=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # lag_prediction(correct=0, d=0, idx=0, idx_valid=-1, len_s=0, n=2, packed=0, s=0, safe=-1, symbols_size=2, valid=1, winner=0)
        pass

def test_keller_lag_prediction_line_531():
    """Formal verification counterexample for lag_prediction (line 531)
    Defect class: python-index-out-of-bounds
    Witness: [correct=0, d=0, idx=0, idx_valid=0, len_0=0, n=1, packed=0, s=0, safe=0, symbols_size=1, valid=1, winner=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # lag_prediction(correct=0, d=0, idx=0, idx_valid=0, len_0=0, n=1, packed=0, s=0, safe=0, symbols_size=1, valid=1, winner=0)
        pass

def test_keller_lag_prediction_line_534():
    """Formal verification counterexample for lag_prediction (line 534)
    Defect class: python-zero-division
    Witness: [c=0, correct=0, d=0, idx=0, idx_valid=0, n=1, n_pred=0, p_global=0, packed=0, s=0, safe=0, symbols_size=1, valid=1, winner=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(ZeroDivisionError):
        # Call target under witness parameters
        # lag_prediction(c=0, correct=0, d=0, idx=0, idx_valid=0, n=1, n_pred=0, p_global=0, packed=0, s=0, safe=0, symbols_size=1, valid=1, winner=0)
        pass

def test_keller_multi_mmc_prediction_line_555():
    """Formal verification counterexample for multi_mmc_prediction (line 555)
    Defect class: python-index-out-of-bounds
    Witness: [d=0, dict_0=-1, len_list=0, n=3, n_pred=1, symbols_size=3]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # multi_mmc_prediction(d=0, dict_0=-1, len_list=0, n=3, n_pred=1, symbols_size=3)
        pass

def test_keller_multi_mmc_prediction_line_555():
    """Formal verification counterexample for multi_mmc_prediction (line 555)
    Defect class: python-index-out-of-bounds
    Witness: [d=0, len_dict=0, n=2, n_pred=0, symbols_size=2]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # multi_mmc_prediction(d=0, len_dict=0, n=2, n_pred=0, symbols_size=2)
        pass

def test_keller_multi_mmc_prediction_line_555():
    """Formal verification counterexample for multi_mmc_prediction (line 555)
    Defect class: python-index-out-of-bounds
    Witness: [d=0, len_dict=0, n=2, n_pred=0, symbols_size=2]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # multi_mmc_prediction(d=0, len_dict=0, n=2, n_pred=0, symbols_size=2)
        pass

def test_keller_multi_mmc_prediction_line_560():
    """Formal verification counterexample for multi_mmc_prediction (line 560)
    Defect class: python-index-out-of-bounds
    Witness: [Optional_int=-1, d=0, entries=0, len_list=0, n=3, n_pred=1, s=0, scoreboard=0, symbols_size=3, winner=1]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # multi_mmc_prediction(Optional_int=-1, d=0, entries=0, len_list=0, n=3, n_pred=1, s=0, scoreboard=0, symbols_size=3, winner=1)
        pass

def test_keller_multi_mmc_prediction_line_560():
    """Formal verification counterexample for multi_mmc_prediction (line 560)
    Defect class: python-index-out-of-bounds
    Witness: [d=0, entries=0, int=-1, len_Optional=0, n=3, n_pred=1, s=0, scoreboard=0, symbols_size=3, winner=1]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # multi_mmc_prediction(d=0, entries=0, int=-1, len_Optional=0, n=3, n_pred=1, s=0, scoreboard=0, symbols_size=3, winner=1)
        pass

def test_keller_multi_mmc_prediction_line_565():
    """Formal verification counterexample for multi_mmc_prediction (line 565)
    Defect class: python-index-out-of-bounds
    Witness: [correct=0, ctx=0, d=0, entries=0, i=1, k=-1, len_s=0, n=2, n_pred=0, s=0, scoreboard=0, symbols_size=2, winner=1]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # multi_mmc_prediction(correct=0, ctx=0, d=0, entries=0, i=1, k=-1, len_s=0, n=2, n_pred=0, s=0, scoreboard=0, symbols_size=2, winner=1)
        pass

def test_keller_multi_mmc_prediction_line_566():
    """Formal verification counterexample for multi_mmc_prediction (line 566)
    Defect class: python-index-out-of-bounds
    Witness: [correct=0, ctx=0, d=0, entries=0, i=0, k=-2, len_s=0, n=3, n_pred=1, s=0, scoreboard=0, symbols_size=3, winner=1, y=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # multi_mmc_prediction(correct=0, ctx=0, d=0, entries=0, i=0, k=-2, len_s=0, n=3, n_pred=1, s=0, scoreboard=0, symbols_size=3, winner=1, y=0)
        pass

def test_keller_multi_mmc_prediction_line_567():
    """Formal verification counterexample for multi_mmc_prediction (line 567)
    Defect class: python-index-out-of-bounds
    Witness: [correct=0, ctx=0, d=0, entries=0, i=1, k=-1, len_models=0, m=0, models_k=0, n=3, n_pred=1, s=0, scoreboard=0, symbols_size=3, winner=1, y=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # multi_mmc_prediction(correct=0, ctx=0, d=0, entries=0, i=1, k=-1, len_models=0, m=0, models_k=0, n=3, n_pred=1, s=0, scoreboard=0, symbols_size=3, winner=1, y=0)
        pass

def test_keller_multi_mmc_prediction_line_571():
    """Formal verification counterexample for multi_mmc_prediction (line 571)
    Defect class: python-index-out-of-bounds
    Witness: [correct=0, ctx=0, d=0, entries=0, entries_k=0, i=1, k=-1, len_m=0, m=0, max_entries=1, models_k=0, n=2, n_pred=0, s=0, scoreboard=0, sub=0, symbols_size=2, winner=1, y=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # multi_mmc_prediction(correct=0, ctx=0, d=0, entries=0, entries_k=0, i=1, k=-1, len_m=0, m=0, max_entries=1, models_k=0, n=2, n_pred=0, s=0, scoreboard=0, sub=0, symbols_size=2, winner=1, y=0)
        pass

def test_keller_multi_mmc_prediction_line_572():
    """Formal verification counterexample for multi_mmc_prediction (line 572)
    Defect class: python-index-out-of-bounds
    Witness: [correct=0, ctx=0, d=0, entries=0, entries_k=0, i=1, k=-1, len_entries=0, m=0, max_entries=1, models_k=0, n=3, n_pred=1, s=0, scoreboard=0, sub=0, symbols_size=3, winner=1, y=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # multi_mmc_prediction(correct=0, ctx=0, d=0, entries=0, entries_k=0, i=1, k=-1, len_entries=0, m=0, max_entries=1, models_k=0, n=3, n_pred=1, s=0, scoreboard=0, sub=0, symbols_size=3, winner=1, y=0)
        pass

def test_keller_multi_mmc_prediction_line_577():
    """Formal verification counterexample for multi_mmc_prediction (line 577)
    Defect class: python-index-out-of-bounds
    Witness: [correct=0, ctx=0, d=0, entries=0, i=1, k=-1, len_s=0, n=2, n_pred=0, s=0, scoreboard=0, symbols_size=2, winner=1]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # multi_mmc_prediction(correct=0, ctx=0, d=0, entries=0, i=1, k=-1, len_s=0, n=2, n_pred=0, s=0, scoreboard=0, symbols_size=2, winner=1)
        pass

def test_keller_multi_mmc_prediction_line_578():
    """Formal verification counterexample for multi_mmc_prediction (line 578)
    Defect class: python-index-out-of-bounds
    Witness: [correct=0, ctx=0, d=0, entries=0, i=1, k=-1, len_models=0, m=0, models_k=0, n=3, n_pred=1, s=0, scoreboard=0, symbols_size=3, winner=1]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # multi_mmc_prediction(correct=0, ctx=0, d=0, entries=0, i=1, k=-1, len_models=0, m=0, models_k=0, n=3, n_pred=1, s=0, scoreboard=0, symbols_size=3, winner=1)
        pass

def test_keller_multi_mmc_prediction_line_580():
    """Formal verification counterexample for multi_mmc_prediction (line 580)
    Defect class: python-index-out-of-bounds
    Witness: [correct=0, ctx=0, d=0, entries=0, i=1, k=-1, len_subpredict=0, m=0, models_k=0, n=3, n_pred=1, s=0, scoreboard=0, sub=0, symbols_size=3, winner=1]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # multi_mmc_prediction(correct=0, ctx=0, d=0, entries=0, i=1, k=-1, len_subpredict=0, m=0, models_k=0, n=3, n_pred=1, s=0, scoreboard=0, sub=0, symbols_size=3, winner=1)
        pass

def test_keller_multi_mmc_prediction_line_580():
    """Formal verification counterexample for multi_mmc_prediction (line 580)
    Defect class: python-index-out-of-bounds
    Witness: [correct=0, ctx=0, d=0, entries=0, i=1, k=-1, len_sub=0, m=0, models_k=0, n=3, n_pred=1, s=0, scoreboard=0, sub=0, symbols_size=3, winner=1, y=-1]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # multi_mmc_prediction(correct=0, ctx=0, d=0, entries=0, i=1, k=-1, len_sub=0, m=0, models_k=0, n=3, n_pred=1, s=0, scoreboard=0, sub=0, symbols_size=3, winner=1, y=-1)
        pass

def test_keller_multi_mmc_prediction_line_582():
    """Formal verification counterexample for multi_mmc_prediction (line 582)
    Defect class: python-index-out-of-bounds
    Witness: [correct=0, d=0, entries=0, i=0, k=-1, len_subpredict=0, n=3, n_pred=1, s=0, scoreboard=0, symbols_size=3, winner=1]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # multi_mmc_prediction(correct=0, d=0, entries=0, i=0, k=-1, len_subpredict=0, n=3, n_pred=1, s=0, scoreboard=0, symbols_size=3, winner=1)
        pass

def test_keller_multi_mmc_prediction_line_587():
    """Formal verification counterexample for multi_mmc_prediction (line 587)
    Defect class: python-index-out-of-bounds
    Witness: [correct=0, d=0, entries=0, k=-1, len_scoreboard=0, n=3, n_pred=1, s=0, scoreboard=0, subpredict_k=0, symbols_size=3, winner=1]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # multi_mmc_prediction(correct=0, d=0, entries=0, k=-1, len_scoreboard=0, n=3, n_pred=1, s=0, scoreboard=0, subpredict_k=0, symbols_size=3, winner=1)
        pass

def test_keller_multi_mmc_prediction_line_591():
    """Formal verification counterexample for multi_mmc_prediction (line 591)
    Defect class: python-zero-division
    Witness: [c=0, correct=0, d=0, entries=0, n=2, n_pred=0, p_global=0, s=0, scoreboard=0, symbols_size=2, winner=1]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(ZeroDivisionError):
        # Call target under witness parameters
        # multi_mmc_prediction(c=0, correct=0, d=0, entries=0, n=2, n_pred=0, p_global=0, s=0, scoreboard=0, symbols_size=2, winner=1)
        pass

def test_keller_estimate_min_entropy_line_643():
    """Formal verification counterexample for estimate_min_entropy (line 643)
    Defect class: python-index-out-of-bounds
    Witness: [len_symbols=0, limit=0, symbols=0, symbols_0=0, symbols_size=1]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # estimate_min_entropy(len_symbols=0, limit=0, symbols=0, symbols_0=0, symbols_size=1)
        pass

def test_keller_estimate_min_entropy_line_644():
    """Formal verification counterexample for estimate_min_entropy (line 644)
    Defect class: python-index-out-of-bounds
    Witness: [len_dict=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # estimate_min_entropy(len_dict=0)
        pass

def test_keller_estimate_min_entropy_line_646():
    """Formal verification counterexample for estimate_min_entropy (line 646)
    Defect class: python-index-out-of-bounds
    Witness: [bits_per_symbol=1, len_results=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # estimate_min_entropy(bits_per_symbol=1, len_results=0)
        pass

def test_keller_estimate_min_entropy_line_647():
    """Formal verification counterexample for estimate_min_entropy (line 647)
    Defect class: python-index-out-of-bounds
    Witness: [bits_per_symbol=1, len_results=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # estimate_min_entropy(bits_per_symbol=1, len_results=0)
        pass

def test_keller_estimate_min_entropy_line_648():
    """Formal verification counterexample for estimate_min_entropy (line 648)
    Defect class: python-index-out-of-bounds
    Witness: [bits_per_symbol=1, len_results=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # estimate_min_entropy(bits_per_symbol=1, len_results=0)
        pass

def test_keller_estimate_min_entropy_line_649():
    """Formal verification counterexample for estimate_min_entropy (line 649)
    Defect class: python-index-out-of-bounds
    Witness: [len_results=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # estimate_min_entropy(len_results=0)
        pass

def test_keller_estimate_min_entropy_line_650():
    """Formal verification counterexample for estimate_min_entropy (line 650)
    Defect class: python-index-out-of-bounds
    Witness: [len_results=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # estimate_min_entropy(len_results=0)
        pass

def test_keller_estimate_min_entropy_line_651():
    """Formal verification counterexample for estimate_min_entropy (line 651)
    Defect class: python-index-out-of-bounds
    Witness: [len_results=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # estimate_min_entropy(len_results=0)
        pass

def test_keller_estimate_min_entropy_line_652():
    """Formal verification counterexample for estimate_min_entropy (line 652)
    Defect class: python-index-out-of-bounds
    Witness: [len_results=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # estimate_min_entropy(len_results=0)
        pass

def test_keller_estimate_min_entropy_line_653():
    """Formal verification counterexample for estimate_min_entropy (line 653)
    Defect class: python-index-out-of-bounds
    Witness: [len_results=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # estimate_min_entropy(len_results=0)
        pass

def test_keller_estimate_min_entropy_line_654():
    """Formal verification counterexample for estimate_min_entropy (line 654)
    Defect class: python-index-out-of-bounds
    Witness: [len_results=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # estimate_min_entropy(len_results=0)
        pass

def test_keller_estimate_min_entropy_line_655():
    """Formal verification counterexample for estimate_min_entropy (line 655)
    Defect class: python-index-out-of-bounds
    Witness: [len_results=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # estimate_min_entropy(len_results=0)
        pass

def test_keller_true_quantized_gaussian_entropy_line_101():
    """Formal verification counterexample for true_quantized_gaussian_entropy (line 101)
    Defect class: python-zero-division
    Witness: [bits=-2, levels=0, sigma=0.0, width=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(ZeroDivisionError):
        # Call target under witness parameters
        # true_quantized_gaussian_entropy(bits=-2, levels=0, sigma=0.0, width=0)
        pass

def test_keller_true_quantized_gaussian_entropy_line_102():
    """Formal verification counterexample for true_quantized_gaussian_entropy (line 102)
    Defect class: python-zero-division
    Witness: [bits=-2, edges=0, levels=0, sigma=0.0, width=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(ZeroDivisionError):
        # Call target under witness parameters
        # true_quantized_gaussian_entropy(bits=-2, edges=0, levels=0, sigma=0.0, width=0)
        pass

def test_keller_true_quantized_gaussian_entropy_line_103():
    """Formal verification counterexample for true_quantized_gaussian_entropy (line 103)
    Defect class: python-zero-division
    Witness: [bits=-2, div0=(x!1, edges=0, levels=0, mod0=(x!1, p=0, sigma=0.0, width=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(ZeroDivisionError):
        # Call target under witness parameters
        # true_quantized_gaussian_entropy(bits=-2, div0=(x!1, edges=0, levels=0, mod0=(x!1, p=0, sigma=0.0, width=0)
        pass

def test_keller_true_quantized_gaussian_entropy_line_105():
    """Formal verification counterexample for true_quantized_gaussian_entropy (line 105)
    Defect class: python-index-out-of-bounds
    Witness: [bits=-2, div0=(x!1, edges=0, len_p=0, levels=0, mod0=(x!1, p=0, sigma=0.0, width=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # true_quantized_gaussian_entropy(bits=-2, div0=(x!1, edges=0, len_p=0, levels=0, mod0=(x!1, p=0, sigma=0.0, width=0)
        pass

def test_keller_true_quantized_gaussian_entropy_line_105():
    """Formal verification counterexample for true_quantized_gaussian_entropy (line 105)
    Defect class: python-zero-division
    Witness: [bits=-2, div0=(x!1, edges=0, levels=0, mod0=(x!1, p=0, sigma=0.0, width=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(ZeroDivisionError):
        # Call target under witness parameters
        # true_quantized_gaussian_entropy(bits=-2, div0=(x!1, edges=0, levels=0, mod0=(x!1, p=0, sigma=0.0, width=0)
        pass

def test_keller_true_quantized_gaussian_entropy_line_105():
    """Formal verification counterexample for true_quantized_gaussian_entropy (line 105)
    Defect class: python-index-out-of-bounds
    Witness: [bits=-2, div0=(x!1, edges=0, len_edges=0, levels=0, mod0=(x!1, p=0, sigma=0.0, width=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # true_quantized_gaussian_entropy(bits=-2, div0=(x!1, edges=0, len_edges=0, levels=0, mod0=(x!1, p=0, sigma=0.0, width=0)
        pass

def test_keller_true_quantized_gaussian_entropy_line_106():
    """Formal verification counterexample for true_quantized_gaussian_entropy (line 106)
    Defect class: python-index-out-of-bounds
    Witness: [bits=-2, div0=(x!1, edges=0, len_p=0, levels=0, mod0=(x!1, p=0, sigma=0.0, width=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # true_quantized_gaussian_entropy(bits=-2, div0=(x!1, edges=0, len_p=0, levels=0, mod0=(x!1, p=0, sigma=0.0, width=0)
        pass

def test_keller_true_quantized_gaussian_entropy_line_106():
    """Formal verification counterexample for true_quantized_gaussian_entropy (line 106)
    Defect class: python-zero-division
    Witness: [bits=-2, div0=(x!1, edges=0, levels=0, mod0=(x!1, p=0, sigma=0.0, width=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(ZeroDivisionError):
        # Call target under witness parameters
        # true_quantized_gaussian_entropy(bits=-2, div0=(x!1, edges=0, levels=0, mod0=(x!1, p=0, sigma=0.0, width=0)
        pass

def test_keller_true_quantized_gaussian_entropy_line_106():
    """Formal verification counterexample for true_quantized_gaussian_entropy (line 106)
    Defect class: python-index-out-of-bounds
    Witness: [bits=-2, div0=(x!1, edges=0, len_edges=0, levels=0, mod0=(x!1, p=0, sigma=0.0, width=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # true_quantized_gaussian_entropy(bits=-2, div0=(x!1, edges=0, len_edges=0, levels=0, mod0=(x!1, p=0, sigma=0.0, width=0)
        pass

def test_keller_encode_line_130():
    """Formal verification counterexample for encode (line 130)
    Defect class: python-zero-division
    Witness: [block_len=0, k=0, len_padded=0, padded=0, payload=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(ZeroDivisionError):
        # Call target under witness parameters
        # encode(block_len=0, k=0, len_padded=0, padded=0, payload=0)
        pass

def test_keller_encode_line_131():
    """Formal verification counterexample for encode (line 131)
    Defect class: python-index-out-of-bounds
    Witness: [block_len=0, blocks=0, k=0, len_padded=0, padded=0, payload=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # encode(block_len=0, blocks=0, k=0, len_padded=0, padded=0, payload=0)
        pass

def test_keller_encode_line_137():
    """Formal verification counterexample for encode (line 137)
    Defect class: python-index-out-of-bounds
    Witness: [Shard=6, block_len=0, blocks=0, dst=0, k=0, len_list=0, len_padded=0, mat=0, padded=0, payload=0, src=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # encode(Shard=6, block_len=0, blocks=0, dst=0, k=0, len_list=0, len_padded=0, mat=0, padded=0, payload=0, src=0)
        pass

def test_keller_encode_line_139():
    """Formal verification counterexample for encode (line 139)
    Defect class: python-index-out-of-bounds
    Witness: [block_len=0, blocks=0, dst=0, k=0, len_mat=0, len_padded=0, m=6, mat=0, mat_m=0, padded=0, payload=0, row=0, src=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # encode(block_len=0, blocks=0, dst=0, k=0, len_mat=0, len_padded=0, m=6, mat=0, mat_m=0, padded=0, payload=0, row=0, src=0)
        pass

def test_keller_encode_line_144():
    """Formal verification counterexample for encode (line 144)
    Defect class: python-index-out-of-bounds
    Witness: [acc=0, block_len=0, blocks=0, dst=0, i=6, k=0, len_padded=0, len_row=0, mat=0, mat_m=0, out=0, padded=0, payload=0, row=0, src=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # encode(acc=0, block_len=0, blocks=0, dst=0, i=6, k=0, len_padded=0, len_row=0, mat=0, mat_m=0, out=0, padded=0, payload=0, row=0, src=0)
        pass

def test_keller_encode_line_144():
    """Formal verification counterexample for encode (line 144)
    Defect class: python-index-out-of-bounds
    Witness: [acc=0, block_len=0, blocks=0, dst=0, j=6, k=0, len_blocks_i=0, len_padded=0, mat=0, mat_m=0, out=0, padded=0, payload=0, row=0, src=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # encode(acc=0, block_len=0, blocks=0, dst=0, j=6, k=0, len_blocks_i=0, len_padded=0, mat=0, mat_m=0, out=0, padded=0, payload=0, row=0, src=0)
        pass

def test_keller_encode_line_144():
    """Formal verification counterexample for encode (line 144)
    Defect class: python-index-out-of-bounds
    Witness: [acc=0, block_len=0, blocks=0, dst=0, i=6, k=0, len_blocks=0, len_padded=0, mat=0, mat_m=0, out=0, padded=0, payload=0, row=0, src=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # encode(acc=0, block_len=0, blocks=0, dst=0, i=6, k=0, len_blocks=0, len_padded=0, mat=0, mat_m=0, out=0, padded=0, payload=0, row=0, src=0)
        pass

def test_keller_encode_line_145():
    """Formal verification counterexample for encode (line 145)
    Defect class: python-index-out-of-bounds
    Witness: [acc=0, block_len=0, blocks=0, dst=0, j=6, k=0, len_out=0, len_padded=0, mat=0, mat_m=0, out=0, padded=0, payload=0, row=0, src=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # encode(acc=0, block_len=0, blocks=0, dst=0, j=6, k=0, len_out=0, len_padded=0, mat=0, mat_m=0, out=0, padded=0, payload=0, row=0, src=0)
        pass

def test_keller_decode_line_185():
    """Formal verification counterexample for decode (line 185)
    Defect class: python-index-out-of-bounds
    Witness: [bodies=0, dst=0, indices=0, len_0=0, lengths=0, mat=0, n_all=0, src=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # decode(bodies=0, dst=0, indices=0, len_0=0, lengths=0, mat=0, n_all=0, src=0)
        pass

def test_keller_decode_line_185():
    """Formal verification counterexample for decode (line 185)
    Defect class: python-index-out-of-bounds
    Witness: [bodies=0, dst=0, indices=0, len_b=0, lengths=0, mat=0, n_all=0, src=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # decode(bodies=0, dst=0, indices=0, len_b=0, lengths=0, mat=0, n_all=0, src=0)
        pass

def test_keller_decode_line_191():
    """Formal verification counterexample for decode (line 191)
    Defect class: python-index-out-of-bounds
    Witness: [_HEADER_size=0, block_len=0, bodies=0, dst=0, indices=0, len_bodies=0, len_bodies_0=0, lengths=0, mat=0, n_all=0, orig_len=0, src=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # decode(_HEADER_size=0, block_len=0, bodies=0, dst=0, indices=0, len_bodies=0, len_bodies_0=0, lengths=0, mat=0, n_all=0, orig_len=0, src=0)
        pass

def test_keller_decode_line_197():
    """Formal verification counterexample for decode (line 197)
    Defect class: python-index-out-of-bounds
    Witness: [_HEADER_size=0, block_len=0, blocks_out=0, bodies=0, dst=0, i=-1, indices=0, len_bodies_0=0, len_mat=0, lengths=0, mat=0, mat_i=0, n_all=0, orig_len=0, row=0, src=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # decode(_HEADER_size=0, block_len=0, blocks_out=0, bodies=0, dst=0, i=-1, indices=0, len_bodies_0=0, len_mat=0, lengths=0, mat=0, mat_i=0, n_all=0, orig_len=0, row=0, src=0)
        pass

def test_keller_decode_line_201():
    """Formal verification counterexample for decode (line 201)
    Defect class: python-index-out-of-bounds
    Witness: [_HEADER_size=0, acc=0, block_len=0, blocks_out=0, bodies=0, dst=0, indices=0, len_bodies_0=0, len_row=0, lengths=0, m=-1, mat=0, mat_i=0, n_all=0, orig_len=0, row=0, src=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # decode(_HEADER_size=0, acc=0, block_len=0, blocks_out=0, bodies=0, dst=0, indices=0, len_bodies_0=0, len_row=0, lengths=0, m=-1, mat=0, mat_i=0, n_all=0, orig_len=0, row=0, src=0)
        pass

def test_keller_decode_line_201():
    """Formal verification counterexample for decode (line 201)
    Defect class: python-index-out-of-bounds
    Witness: [_HEADER_size=0, acc=0, block_len=0, blocks_out=0, bodies=0, dst=0, indices=0, j=-1, len_bodies_0=0, len_bodies_m=0, lengths=0, mat=0, mat_i=0, n_all=0, orig_len=0, row=0, src=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # decode(_HEADER_size=0, acc=0, block_len=0, blocks_out=0, bodies=0, dst=0, indices=0, j=-1, len_bodies_0=0, len_bodies_m=0, lengths=0, mat=0, mat_i=0, n_all=0, orig_len=0, row=0, src=0)
        pass

def test_keller_decode_line_201():
    """Formal verification counterexample for decode (line 201)
    Defect class: python-index-out-of-bounds
    Witness: [_HEADER_size=0, acc=0, block_len=0, blocks_out=0, bodies=0, dst=0, indices=0, len_bodies=0, len_bodies_0=0, lengths=0, m=-1, mat=0, mat_i=0, n_all=0, orig_len=0, row=0, src=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # decode(_HEADER_size=0, acc=0, block_len=0, blocks_out=0, bodies=0, dst=0, indices=0, len_bodies=0, len_bodies_0=0, lengths=0, m=-1, mat=0, mat_i=0, n_all=0, orig_len=0, row=0, src=0)
        pass

def test_keller_decode_line_202():
    """Formal verification counterexample for decode (line 202)
    Defect class: python-index-out-of-bounds
    Witness: [_HEADER_size=0, acc=0, block_len=0, blocks_out=0, bodies=0, dst=0, indices=0, j=-1, len_blocks_out_i=0, len_bodies_0=0, lengths=0, mat=0, mat_i=0, n_all=0, orig_len=0, row=0, src=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # decode(_HEADER_size=0, acc=0, block_len=0, blocks_out=0, bodies=0, dst=0, indices=0, j=-1, len_blocks_out_i=0, len_bodies_0=0, lengths=0, mat=0, mat_i=0, n_all=0, orig_len=0, row=0, src=0)
        pass

def test_keller_decode_line_202():
    """Formal verification counterexample for decode (line 202)
    Defect class: python-index-out-of-bounds
    Witness: [_HEADER_size=0, acc=0, block_len=0, blocks_out=0, bodies=0, dst=0, i=-1, indices=0, len_blocks_out=0, len_bodies_0=0, lengths=0, mat=0, mat_i=0, n_all=0, orig_len=0, row=0, src=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # decode(_HEADER_size=0, acc=0, block_len=0, blocks_out=0, bodies=0, dst=0, i=-1, indices=0, len_blocks_out=0, len_bodies_0=0, lengths=0, mat=0, mat_i=0, n_all=0, orig_len=0, row=0, src=0)
        pass

def test_keller_decode_line_206():
    """Formal verification counterexample for decode (line 206)
    Defect class: python-index-out-of-bounds
    Witness: [_HEADER_size=0, block_len=0, blocks_out=0, bodies=0, dst=0, indices=0, joined=0, len_bodies_0=0, len_joined=0, lengths=0, mat=0, n_all=0, orig_len=0, src=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # decode(_HEADER_size=0, block_len=0, blocks_out=0, bodies=0, dst=0, indices=0, joined=0, len_bodies_0=0, len_joined=0, lengths=0, mat=0, n_all=0, orig_len=0, src=0)
        pass

def test_keller__pad_payload_line_211():
    """Formal verification counterexample for _pad_payload (line 211)
    Defect class: python-zero-division
    Witness: [k=0, len_payload=0, rem=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(ZeroDivisionError):
        # Call target under witness parameters
        # _pad_payload(k=0, len_payload=0, rem=0)
        pass

def test_keller_gf_mul_line_52():
    """Formal verification counterexample for gf_mul (line 52)
    Defect class: python-index-out-of-bounds
    Witness: [len__ALOG=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # gf_mul(len__ALOG=0)
        pass

def test_keller_gf_mul_line_52():
    """Formal verification counterexample for gf_mul (line 52)
    Defect class: python-index-out-of-bounds
    Witness: [a=-1, len__LOG=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # gf_mul(a=-1, len__LOG=0)
        pass

def test_keller_gf_mul_line_52():
    """Formal verification counterexample for gf_mul (line 52)
    Defect class: python-index-out-of-bounds
    Witness: [b=-1, len__LOG=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # gf_mul(b=-1, len__LOG=0)
        pass

def test_keller_gf_div_line_62():
    """Formal verification counterexample for gf_div (line 62)
    Defect class: python-index-out-of-bounds
    Witness: [len__ALOG=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # gf_div(len__ALOG=0)
        pass

def test_keller_gf_div_line_62():
    """Formal verification counterexample for gf_div (line 62)
    Defect class: python-index-out-of-bounds
    Witness: [a=-1, len__LOG=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # gf_div(a=-1, len__LOG=0)
        pass

def test_keller_gf_div_line_62():
    """Formal verification counterexample for gf_div (line 62)
    Defect class: python-index-out-of-bounds
    Witness: [b=-1, len__LOG=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # gf_div(b=-1, len__LOG=0)
        pass

def test_keller_gf_pow_line_70():
    """Formal verification counterexample for gf_pow (line 70)
    Defect class: python-index-out-of-bounds
    Witness: [len__ALOG=0, n=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # gf_pow(len__ALOG=0, n=0)
        pass

def test_keller_gf_pow_line_70():
    """Formal verification counterexample for gf_pow (line 70)
    Defect class: python-index-out-of-bounds
    Witness: [a=-1, len__LOG=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # gf_pow(a=-1, len__LOG=0)
        pass

def test_keller_HardenedEntropy_from_dict_line_89():
    """Formal verification counterexample for HardenedEntropy.from_dict (line 89)
    Defect class: python-index-out-of-bounds
    Witness: [d_0=0, len_d=0, p=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # HardenedEntropy.from_dict(d_0=0, len_d=0, p=0)
        pass

def test_keller_HardenedEntropy_from_dict_line_99():
    """Formal verification counterexample for HardenedEntropy.from_dict (line 99)
    Defect class: python-index-out-of-bounds
    Witness: [d_0=0, len_d=0, p=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # HardenedEntropy.from_dict(d_0=0, len_d=0, p=0)
        pass

def test_keller_HardenedEntropy_from_dict_line_98():
    """Formal verification counterexample for HardenedEntropy.from_dict (line 98)
    Defect class: python-index-out-of-bounds
    Witness: [d_0=0, len_d=0, p=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # HardenedEntropy.from_dict(d_0=0, len_d=0, p=0)
        pass

def test_keller_HardenedEntropy_from_dict_line_92():
    """Formal verification counterexample for HardenedEntropy.from_dict (line 92)
    Defect class: python-index-out-of-bounds
    Witness: [d_0=0, len_p=0, p=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # HardenedEntropy.from_dict(d_0=0, len_p=0, p=0)
        pass

def test_keller_HardenedEntropy_from_dict_line_93():
    """Formal verification counterexample for HardenedEntropy.from_dict (line 93)
    Defect class: python-index-out-of-bounds
    Witness: [d_0=0, len_p=0, p=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # HardenedEntropy.from_dict(d_0=0, len_p=0, p=0)
        pass

def test_keller_HardenedEntropy_from_dict_line_97():
    """Formal verification counterexample for HardenedEntropy.from_dict (line 97)
    Defect class: python-index-out-of-bounds
    Witness: [d_0=0, len_d=0, p=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # HardenedEntropy.from_dict(d_0=0, len_d=0, p=0)
        pass

def test_keller_HardenedEntropy_from_dict_line_94():
    """Formal verification counterexample for HardenedEntropy.from_dict (line 94)
    Defect class: python-index-out-of-bounds
    Witness: [d_0=0, len_p=0, p=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # HardenedEntropy.from_dict(d_0=0, len_p=0, p=0)
        pass

def test_keller_HardenedEntropy_from_dict_line_95():
    """Formal verification counterexample for HardenedEntropy.from_dict (line 95)
    Defect class: python-index-out-of-bounds
    Witness: [d_0=0, len_p=0, p=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # HardenedEntropy.from_dict(d_0=0, len_p=0, p=0)
        pass

def test_keller_polynomial_embed_line_123():
    """Formal verification counterexample for polynomial_embed (line 123)
    Defect class: python-index-out-of-bounds
    Witness: [b=0, coeffs=0, len_coeffs=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # polynomial_embed(b=0, coeffs=0, len_coeffs=0)
        pass

def test_keller_polynomial_embed_line_123():
    """Formal verification counterexample for polynomial_embed (line 123)
    Defect class: python-index-out-of-bounds
    Witness: [b=0, coeffs=0, len_b=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # polynomial_embed(b=0, coeffs=0, len_b=0)
        pass

def test_keller_EntropyCertificate_from_dict_line_83():
    """Formal verification counterexample for EntropyCertificate.from_dict (line 83)
    Defect class: python-index-out-of-bounds
    Witness: [c=0, d_0=0, len_d=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # EntropyCertificate.from_dict(c=0, d_0=0, len_d=0)
        pass

def test_keller_EntropyCertificate_from_dict_line_85():
    """Formal verification counterexample for EntropyCertificate.from_dict (line 85)
    Defect class: python-index-out-of-bounds
    Witness: [c=0, d_0=0, len_d=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # EntropyCertificate.from_dict(c=0, d_0=0, len_d=0)
        pass

def test_keller_EntropyCertificate_from_dict_line_86():
    """Formal verification counterexample for EntropyCertificate.from_dict (line 86)
    Defect class: python-index-out-of-bounds
    Witness: [c=0, d_0=0, len_d=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # EntropyCertificate.from_dict(c=0, d_0=0, len_d=0)
        pass

def test_keller_EntropyCertificate_from_dict_line_87():
    """Formal verification counterexample for EntropyCertificate.from_dict (line 87)
    Defect class: python-index-out-of-bounds
    Witness: [c=0, d_0=0, len_d=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # EntropyCertificate.from_dict(c=0, d_0=0, len_d=0)
        pass

def test_keller_EntropyCertificate_from_dict_line_96():
    """Formal verification counterexample for EntropyCertificate.from_dict (line 96)
    Defect class: python-index-out-of-bounds
    Witness: [c=0, d_0=0, len_d=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # EntropyCertificate.from_dict(c=0, d_0=0, len_d=0)
        pass

def test_keller_EntropyCertificate_from_dict_line_97():
    """Formal verification counterexample for EntropyCertificate.from_dict (line 97)
    Defect class: python-index-out-of-bounds
    Witness: [c=0, d_0=0, len_d=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # EntropyCertificate.from_dict(c=0, d_0=0, len_d=0)
        pass

def test_keller_EntropyCertificate_from_dict_line_89():
    """Formal verification counterexample for EntropyCertificate.from_dict (line 89)
    Defect class: python-index-out-of-bounds
    Witness: [c=0, d_0=0, len_c=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # EntropyCertificate.from_dict(c=0, d_0=0, len_c=0)
        pass

def test_keller_EntropyCertificate_from_dict_line_90():
    """Formal verification counterexample for EntropyCertificate.from_dict (line 90)
    Defect class: python-index-out-of-bounds
    Witness: [c=0, d_0=0, len_c=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # EntropyCertificate.from_dict(c=0, d_0=0, len_c=0)
        pass

def test_keller_EntropyCertificate_from_dict_line_91():
    """Formal verification counterexample for EntropyCertificate.from_dict (line 91)
    Defect class: python-index-out-of-bounds
    Witness: [c=0, d_0=0, len_c=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # EntropyCertificate.from_dict(c=0, d_0=0, len_c=0)
        pass

def test_keller_EntropyCertificate_from_dict_line_92():
    """Formal verification counterexample for EntropyCertificate.from_dict (line 92)
    Defect class: python-index-out-of-bounds
    Witness: [c=0, d_0=0, len_c=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # EntropyCertificate.from_dict(c=0, d_0=0, len_c=0)
        pass

def test_keller_EntropyCertificate_from_dict_line_93():
    """Formal verification counterexample for EntropyCertificate.from_dict (line 93)
    Defect class: python-index-out-of-bounds
    Witness: [c=0, d_0=0, len_c=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # EntropyCertificate.from_dict(c=0, d_0=0, len_c=0)
        pass

def test_keller_nullifier_line_55():
    """Formal verification counterexample for nullifier (line 55)
    Defect class: python-index-out-of-bounds
    Witness: [digest=0, len_0=0, wid=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # nullifier(digest=0, len_0=0, wid=0)
        pass

def test_keller__field_add_line_69():
    """Formal verification counterexample for _field_add (line 69)
    Defect class: python-zero-division
    Witness: [_PRIME=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(ZeroDivisionError):
        # Call target under witness parameters
        # _field_add(_PRIME=0)
        pass

def test_keller__field_sub_line_73():
    """Formal verification counterexample for _field_sub (line 73)
    Defect class: python-zero-division
    Witness: [_PRIME=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(ZeroDivisionError):
        # Call target under witness parameters
        # _field_sub(_PRIME=0)
        pass

def test_keller__field_mul_line_77():
    """Formal verification counterexample for _field_mul (line 77)
    Defect class: python-zero-division
    Witness: [_PRIME=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(ZeroDivisionError):
        # Call target under witness parameters
        # _field_mul(_PRIME=0)
        pass

def test_keller__split_secret_line_102():
    """Formal verification counterexample for _split_secret (line 102)
    Defect class: python-zero-division
    Witness: [_PRIME=0, coeffs=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(ZeroDivisionError):
        # Call target under witness parameters
        # _split_secret(_PRIME=0, coeffs=0)
        pass

def test_keller__reconstruct_line_123():
    """Formal verification counterexample for _reconstruct (line 123)
    Defect class: python-zero-division
    Witness: [_PRIME=0, secret=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(ZeroDivisionError):
        # Call target under witness parameters
        # _reconstruct(_PRIME=0, secret=0)
        pass

def test_keller_match_orders_line_168():
    """Formal verification counterexample for match_orders (line 168)
    Defect class: python-index-out-of-bounds
    Witness: [dict_0=-1, len_list=0, rng=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # match_orders(dict_0=-1, len_list=0, rng=0)
        pass

def test_keller_match_orders_line_168():
    """Formal verification counterexample for match_orders (line 168)
    Defect class: python-index-out-of-bounds
    Witness: [len_dict=0, rng=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # match_orders(len_dict=0, rng=0)
        pass

def test_keller_match_orders_line_175():
    """Formal verification counterexample for match_orders (line 175)
    Defect class: python-index-out-of-bounds
    Witness: [len_shares_by_agent_j=0, order_order_id=-1, p_shares=0, q_shares=0, rng=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # match_orders(len_shares_by_agent_j=0, order_order_id=-1, p_shares=0, q_shares=0, rng=0)
        pass

def test_keller_match_orders_line_175():
    """Formal verification counterexample for match_orders (line 175)
    Defect class: python-index-out-of-bounds
    Witness: [j=-1, len_shares_by_agent=0, p_shares=0, q_shares=0, rng=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # match_orders(j=-1, len_shares_by_agent=0, p_shares=0, q_shares=0, rng=0)
        pass

def test_keller_match_orders_line_177():
    """Formal verification counterexample for match_orders (line 177)
    Defect class: python-index-out-of-bounds
    Witness: [j=-1, len_p_shares=0, p_shares=0, q_shares=0, rng=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # match_orders(j=-1, len_p_shares=0, p_shares=0, q_shares=0, rng=0)
        pass

def test_keller_match_orders_line_178():
    """Formal verification counterexample for match_orders (line 178)
    Defect class: python-index-out-of-bounds
    Witness: [j=-1, len_q_shares=0, p_shares=0, q_shares=0, rng=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # match_orders(j=-1, len_q_shares=0, p_shares=0, q_shares=0, rng=0)
        pass

def test_keller_match_orders_line_184():
    """Formal verification counterexample for match_orders (line 184)
    Defect class: python-index-out-of-bounds
    Witness: [MatchRecord=-1, len_list=0, price_by_id=0, qty_by_id=0, rng=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # match_orders(MatchRecord=-1, len_list=0, price_by_id=0, qty_by_id=0, rng=0)
        pass

def test_keller_match_orders_line_191():
    """Formal verification counterexample for match_orders (line 191)
    Defect class: python-index-out-of-bounds
    Witness: [bid_order_id=-1, diffs=0, len_shares_by_agent_j=0, price_by_id=0, qty_by_id=0, rng=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # match_orders(bid_order_id=-1, diffs=0, len_shares_by_agent_j=0, price_by_id=0, qty_by_id=0, rng=0)
        pass

def test_keller_match_orders_line_192():
    """Formal verification counterexample for match_orders (line 192)
    Defect class: python-index-out-of-bounds
    Witness: [diffs=0, len_shares_by_agent_j=0, offer_order_id=-1, price_by_id=0, qty_by_id=0, rng=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # match_orders(diffs=0, len_shares_by_agent_j=0, offer_order_id=-1, price_by_id=0, qty_by_id=0, rng=0)
        pass

def test_keller_match_orders_line_191():
    """Formal verification counterexample for match_orders (line 191)
    Defect class: python-index-out-of-bounds
    Witness: [diffs=0, j=-1, len_shares_by_agent=0, price_by_id=0, qty_by_id=0, rng=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # match_orders(diffs=0, j=-1, len_shares_by_agent=0, price_by_id=0, qty_by_id=0, rng=0)
        pass

def test_keller_match_orders_line_192():
    """Formal verification counterexample for match_orders (line 192)
    Defect class: python-index-out-of-bounds
    Witness: [diffs=0, j=-1, len_shares_by_agent=0, price_by_id=0, qty_by_id=0, rng=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # match_orders(diffs=0, j=-1, len_shares_by_agent=0, price_by_id=0, qty_by_id=0, rng=0)
        pass

def test_keller_match_orders_line_198():
    """Formal verification counterexample for match_orders (line 198)
    Defect class: python-index-out-of-bounds
    Witness: [bid_order_id=-1, diff_mod=0, diffs=0, len_qty_by_id=0, price_by_id=0, qty=0, qty_by_id=0, qty_by_id_bid_order_id=0, qty_by_id_offer_order_id=0, rng=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # match_orders(bid_order_id=-1, diff_mod=0, diffs=0, len_qty_by_id=0, price_by_id=0, qty=0, qty_by_id=0, qty_by_id_bid_order_id=0, qty_by_id_offer_order_id=0, rng=0)
        pass

def test_keller_match_orders_line_198():
    """Formal verification counterexample for match_orders (line 198)
    Defect class: python-index-out-of-bounds
    Witness: [diff_mod=0, diffs=0, len_qty_by_id=0, offer_order_id=-1, price_by_id=0, qty=0, qty_by_id=0, qty_by_id_bid_order_id=0, qty_by_id_offer_order_id=0, rng=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # match_orders(diff_mod=0, diffs=0, len_qty_by_id=0, offer_order_id=-1, price_by_id=0, qty=0, qty_by_id=0, qty_by_id_bid_order_id=0, qty_by_id_offer_order_id=0, rng=0)
        pass

def test_keller_match_orders_line_202():
    """Formal verification counterexample for match_orders (line 202)
    Defect class: python-index-out-of-bounds
    Witness: [diff_mod=0, diffs=0, len_price_by_id=0, offer_order_id=-1, price_by_id=0, qty=0, qty_by_id=0, qty_by_id_bid_order_id=0, qty_by_id_offer_order_id=0, rng=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # match_orders(diff_mod=0, diffs=0, len_price_by_id=0, offer_order_id=-1, price_by_id=0, qty=0, qty_by_id=0, qty_by_id_bid_order_id=0, qty_by_id_offer_order_id=0, rng=0)
        pass

def test_keller_Topology___init___line_103():
    """Formal verification counterexample for Topology.__init__ (line 103)
    Defect class: python-index-out-of-bounds
    Witness: [len_dict=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # Topology.__init__(len_dict=0)
        pass

def test_keller_Topology___init___line_103():
    """Formal verification counterexample for Topology.__init__ (line 103)
    Defect class: python-index-out-of-bounds
    Witness: [len_tuple=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # Topology.__init__(len_tuple=0)
        pass

def test_keller_Topology__build_line_121():
    """Formal verification counterexample for Topology._build (line 121)
    Defect class: python-index-out-of-bounds
    Witness: [connect_prob=1.0, len_self_links=0, rng=0, up_len=1, windows=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # Topology._build(connect_prob=1.0, len_self_links=0, rng=0, up_len=1, windows=0)
        pass

def test_keller_Node___init___line_139():
    """Formal verification counterexample for Node.__init__ (line 139)
    Defect class: python-index-out-of-bounds
    Witness: [len_dict=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # Node.__init__(len_dict=0)
        pass

def test_keller_Node___init___line_139():
    """Formal verification counterexample for Node.__init__ (line 139)
    Defect class: python-index-out-of-bounds
    Witness: [len_tuple=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # Node.__init__(len_tuple=0)
        pass

def test_keller_Node___init___line_140():
    """Formal verification counterexample for Node.__init__ (line 140)
    Defect class: python-index-out-of-bounds
    Witness: [DeliveryRecord=-1, len_list=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # Node.__init__(DeliveryRecord=-1, len_list=0)
        pass

def test_keller_Node_accept_line_154():
    """Formal verification counterexample for Node.accept (line 154)
    Defect class: python-index-out-of-bounds
    Witness: [key=0, len_self_buffer=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # Node.accept(key=0, len_self_buffer=0)
        pass

def test_keller__shortest_next_line_178():
    """Formal verification counterexample for _shortest_next (line 178)
    Defect class: python-index-out-of-bounds
    Witness: [len_dict=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # _shortest_next(len_dict=0)
        pass

def test_keller__shortest_next_line_185():
    """Formal verification counterexample for _shortest_next (line 185)
    Defect class: python-index-out-of-bounds
    Witness: [len_prev=0, nxt=-1, prev=-1, queue=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # _shortest_next(len_prev=0, nxt=-1, prev=-1, queue=0)
        pass

def test_keller__shortest_next_line_191():
    """Formal verification counterexample for _shortest_next (line 191)
    Defect class: python-index-out-of-bounds
    Witness: [cur=-1, len_prev=0, prev_cur=-1, queue=0, src=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # _shortest_next(cur=-1, len_prev=0, prev_cur=-1, queue=0, src=0)
        pass

def test_keller_simulate_line_210():
    """Formal verification counterexample for simulate (line 210)
    Defect class: python-index-out-of-bounds
    Witness: [len_nodes_b_src_buffer=0, nodes=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # simulate(len_nodes_b_src_buffer=0, nodes=0)
        pass

def test_keller_simulate_line_210():
    """Formal verification counterexample for simulate (line 210)
    Defect class: python-index-out-of-bounds
    Witness: [b_src=-1, len_nodes=0, nodes=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # simulate(b_src=-1, len_nodes=0, nodes=0)
        pass

def test_keller_simulate_line_213():
    """Formal verification counterexample for simulate (line 213)
    Defect class: python-index-out-of-bounds
    Witness: [len_dict=0, nodes=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # simulate(len_dict=0, nodes=0)
        pass

def test_keller_simulate_line_215():
    """Formal verification counterexample for simulate (line 215)
    Defect class: python-index-out-of-bounds
    Witness: [b_dst=-1, len_total_per_dst=0, nodes=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # simulate(b_dst=-1, len_total_per_dst=0, nodes=0)
        pass

def test_keller_simulate_line_219():
    """Formal verification counterexample for simulate (line 219)
    Defect class: python-index-out-of-bounds
    Witness: [len_dict=0, nodes=0, result=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # simulate(len_dict=0, nodes=0, result=0)
        pass

def test_keller_simulate_line_219():
    """Formal verification counterexample for simulate (line 219)
    Defect class: python-index-out-of-bounds
    Witness: [len_set=0, nodes=0, result=0, str=-1]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # simulate(len_set=0, nodes=0, result=0, str=-1)
        pass

def test_keller_simulate_line_229():
    """Formal verification counterexample for simulate (line 229)
    Defect class: python-index-out-of-bounds
    Witness: [b=0, entry_bundle=0, len_nodes=0, nodes=0, nodes_nxt=0, nxt=0, result=0, target=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # simulate(b=0, entry_bundle=0, len_nodes=0, nodes=0, nodes_nxt=0, nxt=0, result=0, target=0)
        pass

def test_keller_simulate_line_241():
    """Formal verification counterexample for simulate (line 241)
    Defect class: python-index-out-of-bounds
    Witness: [expired=0, k=-1, len_node_buffer=0, nodes=0, result=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # simulate(expired=0, k=-1, len_node_buffer=0, nodes=0, result=0)
        pass

def test_keller_Route___getitem___line_97():
    """Formal verification counterexample for Route.__getitem__ (line 97)
    Defect class: python-index-out-of-bounds
    Witness: [i=-1, len_self_hops=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # Route.__getitem__(i=-1, len_self_hops=0)
        pass

def test_keller__derive_keys_line_121():
    """Formal verification counterexample for _derive_keys (line 121)
    Defect class: python-index-out-of-bounds
    Witness: [ephem_pub=0, info=0, len_material=0, material=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # _derive_keys(ephem_pub=0, info=0, len_material=0, material=0)
        pass

def test_keller__derive_keys_line_121():
    """Formal verification counterexample for _derive_keys (line 121)
    Defect class: python-index-out-of-bounds
    Witness: [ephem_pub=0, info=0, len_material=0, material=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # _derive_keys(ephem_pub=0, info=0, len_material=0, material=0)
        pass

def test_keller__parse_block_line_148():
    """Formal verification counterexample for _parse_block (line 148)
    Defect class: python-index-out-of-bounds
    Witness: [len_plain=0, magic=0, plain_0=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # _parse_block(len_plain=0, magic=0, plain_0=0)
        pass

def test_keller__parse_block_line_151():
    """Formal verification counterexample for _parse_block (line 151)
    Defect class: python-index-out-of-bounds
    Witness: [final=0, len_plain=0, magic=0, plain_0=0, plain_8=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # _parse_block(final=0, len_plain=0, magic=0, plain_0=0, plain_8=0)
        pass

def test_keller__parse_block_line_152():
    """Formal verification counterexample for _parse_block (line 152)
    Defect class: python-index-out-of-bounds
    Witness: [final=0, len_plain=0, magic=0, plain_0=0, plain_8=0, session=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # _parse_block(final=0, len_plain=0, magic=0, plain_0=0, plain_8=0, session=0)
        pass

def test_keller_wrap_line_205():
    """Formal verification counterexample for wrap (line 205)
    Defect class: python-assertion-failure
    Witness: [block=0, rng_state=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(AssertionError):
        # Call target under witness parameters
        # wrap(block=0, rng_state=0)
        pass

def test_keller_unwrap_line_241():
    """Formal verification counterexample for unwrap (line 241)
    Defect class: python-index-out-of-bounds
    Witness: [len_packet=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # unwrap(len_packet=0)
        pass

def test_keller_unwrap_line_242():
    """Formal verification counterexample for unwrap (line 242)
    Defect class: python-index-out-of-bounds
    Witness: [len_packet=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # unwrap(len_packet=0)
        pass

def test_keller_unwrap_line_243():
    """Formal verification counterexample for unwrap (line 243)
    Defect class: python-index-out-of-bounds
    Witness: [len_packet=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # unwrap(len_packet=0)
        pass

def test_keller_unwrap_line_244():
    """Formal verification counterexample for unwrap (line 244)
    Defect class: python-index-out-of-bounds
    Witness: [len_packet=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # unwrap(len_packet=0)
        pass

def test_keller_unwrap_line_263():
    """Formal verification counterexample for unwrap (line 263)
    Defect class: python-index-out-of-bounds
    Witness: [expected=0, len_body=0, shared=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # unwrap(expected=0, len_body=0, shared=0)
        pass

def test_keller_OnionNode___init___line_276():
    """Formal verification counterexample for OnionNode.__init__ (line 276)
    Defect class: python-index-out-of-bounds
    Witness: [len_dict=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # OnionNode.__init__(len_dict=0)
        pass

def test_keller_OnionNode__check_replay_line_293():
    """Formal verification counterexample for OnionNode._check_replay (line 293)
    Defect class: python-index-out-of-bounds
    Witness: [last=0, len_self__seen=0, session=-1]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # OnionNode._check_replay(last=0, len_self__seen=0, session=-1)
        pass

def test_keller_test_shannon_uniform_bytes_is_8_bits_line_28():
    """Formal verification counterexample for test_shannon_uniform_bytes_is_8_bits (line 28)
    Defect class: python-assertion-failure
    Witness: [data=0, h=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(AssertionError):
        # Call target under witness parameters
        # test_shannon_uniform_bytes_is_8_bits(data=0, h=0)
        pass

def test_keller_test_min_entropy_histogram_uniform_near_one_line_55():
    """Formal verification counterexample for test_min_entropy_histogram_uniform_near_one (line 55)
    Defect class: python-assertion-failure
    Witness: [data=0, h=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(AssertionError):
        # Call target under witness parameters
        # test_min_entropy_histogram_uniform_near_one(data=0, h=0)
        pass

def test_keller_test_spectral_analysis_detects_tone_line_85():
    """Formal verification counterexample for test_spectral_analysis_detects_tone (line 85)
    Defect class: python-assertion-failure
    Witness: [sig=0, spec=0, spec_peak_hz=0, t=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(AssertionError):
        # Call target under witness parameters
        # test_spectral_analysis_detects_tone(sig=0, spec=0, spec_peak_hz=0, t=0)
        pass

def test_keller_test_spectral_analysis_detects_tone_line_86():
    """Formal verification counterexample for test_spectral_analysis_detects_tone (line 86)
    Defect class: python-assertion-failure
    Witness: [sig=0, spec=0, spec_peak_hz=-1, t=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(AssertionError):
        # Call target under witness parameters
        # test_spectral_analysis_detects_tone(sig=0, spec=0, spec_peak_hz=-1, t=0)
        pass

def test_keller_test_spectral_analysis_detects_tone_line_87():
    """Formal verification counterexample for test_spectral_analysis_detects_tone (line 87)
    Defect class: python-assertion-failure
    Witness: [sig=0, spec=0, spec_peak_excess_db=6, spec_peak_hz=10000, t=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(AssertionError):
        # Call target under witness parameters
        # test_spectral_analysis_detects_tone(sig=0, spec=0, spec_peak_excess_db=6, spec_peak_hz=10000, t=0)
        pass

def test_keller_test_spectral_analysis_rf_bandwidth_bounded_line_94():
    """Formal verification counterexample for test_spectral_analysis_rf_bandwidth_bounded (line 94)
    Defect class: python-assertion-failure
    Witness: [spec=0, spec_bandwidth_hz=0, src=0, src_sample_rate=0, win=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(AssertionError):
        # Call target under witness parameters
        # test_spectral_analysis_rf_bandwidth_bounded(spec=0, spec_bandwidth_hz=0, src=0, src_sample_rate=0, win=0)
        pass

def test_keller_test_spectral_analysis_rf_bandwidth_bounded_line_98():
    """Formal verification counterexample for test_spectral_analysis_rf_bandwidth_bounded (line 98)
    Defect class: python-zero-division
    Witness: [cdf=0, pw=10, spec=0, spec_bandwidth_hz=-1, spec_psd_db=0, src=0, src_sample_rate=0, win=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(ZeroDivisionError):
        # Call target under witness parameters
        # test_spectral_analysis_rf_bandwidth_bounded(cdf=0, pw=10, spec=0, spec_bandwidth_hz=-1, spec_psd_db=0, src=0, src_sample_rate=0, win=0)
        pass

def test_keller_test_spectral_analysis_rf_bandwidth_bounded_line_99():
    """Formal verification counterexample for test_spectral_analysis_rf_bandwidth_bounded (line 99)
    Defect class: python-index-out-of-bounds
    Witness: [cdf=0, cdf_0=0, frac_in_band=0, len_cdf=0, pw=10, spec=0, spec_bandwidth_hz=-1, spec_psd_db=0, src=0, src_sample_rate=0, win=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # test_spectral_analysis_rf_bandwidth_bounded(cdf=0, cdf_0=0, frac_in_band=0, len_cdf=0, pw=10, spec=0, spec_bandwidth_hz=-1, spec_psd_db=0, src=0, src_sample_rate=0, win=0)
        pass

def test_keller_test_spectral_analysis_rf_bandwidth_bounded_line_100():
    """Formal verification counterexample for test_spectral_analysis_rf_bandwidth_bounded (line 100)
    Defect class: python-index-out-of-bounds
    Witness: [cdf=0, cdf_0=0, frac_below=0, frac_in_band=0, len_cdf=0, pw=10, spec=0, spec_bandwidth_hz=-1, spec_psd_db=0, src=0, src_sample_rate=0, win=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # test_spectral_analysis_rf_bandwidth_bounded(cdf=0, cdf_0=0, frac_below=0, frac_in_band=0, len_cdf=0, pw=10, spec=0, spec_bandwidth_hz=-1, spec_psd_db=0, src=0, src_sample_rate=0, win=0)
        pass

def test_keller_test_spectral_analysis_rf_bandwidth_bounded_line_101():
    """Formal verification counterexample for test_spectral_analysis_rf_bandwidth_bounded (line 101)
    Defect class: python-assertion-failure
    Witness: [cdf=0, cdf_0=0, frac_below=0, frac_in_band=0, pw=10, spec=0, spec_bandwidth_hz=-1, spec_psd_db=0, src=0, src_sample_rate=0, win=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(AssertionError):
        # Call target under witness parameters
        # test_spectral_analysis_rf_bandwidth_bounded(cdf=0, cdf_0=0, frac_below=0, frac_in_band=0, pw=10, spec=0, spec_bandwidth_hz=-1, spec_psd_db=0, src=0, src_sample_rate=0, win=0)
        pass

def test_keller_test_entropy_report_uniform_stream_near_8_bits_line_124():
    """Formal verification counterexample for test_entropy_report_uniform_stream_near_8_bits (line 124)
    Defect class: python-assertion-failure
    Witness: [data=0, rep=0, rep_shannon_bits_per_byte=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(AssertionError):
        # Call target under witness parameters
        # test_entropy_report_uniform_stream_near_8_bits(data=0, rep=0, rep_shannon_bits_per_byte=0)
        pass

def test_keller_test_entropy_report_uniform_stream_near_8_bits_line_125():
    """Formal verification counterexample for test_entropy_report_uniform_stream_near_8_bits (line 125)
    Defect class: python-assertion-failure
    Witness: [data=0, rep=0, rep_min_entropy_histogram_bits_per_byte=7, rep_shannon_bits_per_byte=8]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(AssertionError):
        # Call target under witness parameters
        # test_entropy_report_uniform_stream_near_8_bits(data=0, rep=0, rep_min_entropy_histogram_bits_per_byte=7, rep_shannon_bits_per_byte=8)
        pass

def test_keller_test_entropy_report_uniform_stream_near_8_bits_line_128():
    """Formal verification counterexample for test_entropy_report_uniform_stream_near_8_bits (line 128)
    Defect class: python-assertion-failure
    Witness: [data=0, rep=0, rep_min_entropy_histogram_bits_per_byte=8, rep_nist90b_bits_per_byte=7, rep_shannon_bits_per_byte=8]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(AssertionError):
        # Call target under witness parameters
        # test_entropy_report_uniform_stream_near_8_bits(data=0, rep=0, rep_min_entropy_histogram_bits_per_byte=8, rep_nist90b_bits_per_byte=7, rep_shannon_bits_per_byte=8)
        pass

def test_keller_test_entropy_report_uniform_stream_near_8_bits_line_129():
    """Formal verification counterexample for test_entropy_report_uniform_stream_near_8_bits (line 129)
    Defect class: python-assertion-failure
    Witness: [data=0, rep=0, rep_conservative_min_bits_per_byte=9, rep_min_entropy_histogram_bits_per_byte=8, rep_nist90b_bits_per_byte=8, rep_shannon_bits_per_byte=8]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(AssertionError):
        # Call target under witness parameters
        # test_entropy_report_uniform_stream_near_8_bits(data=0, rep=0, rep_conservative_min_bits_per_byte=9, rep_min_entropy_histogram_bits_per_byte=8, rep_nist90b_bits_per_byte=8, rep_shannon_bits_per_byte=8)
        pass

def test_keller_test_cross_check_conservative_order_line_137():
    """Formal verification counterexample for test_cross_check_conservative_order (line 137)
    Defect class: python-assertion-failure
    Witness: [codes=0, rep=0, rep_min_entropy_histogram_bits_per_bit=1, rep_shannon_bits_per_bit=0, src=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(AssertionError):
        # Call target under witness parameters
        # test_cross_check_conservative_order(codes=0, rep=0, rep_min_entropy_histogram_bits_per_bit=1, rep_shannon_bits_per_bit=0, src=0)
        pass

def test_keller_test_cross_check_conservative_order_line_138():
    """Formal verification counterexample for test_cross_check_conservative_order (line 138)
    Defect class: python-assertion-failure
    Witness: [codes=0, rep=0, rep_min_entropy_histogram_bits_per_bit=0, rep_nist90b_bits_per_bit=1, rep_shannon_bits_per_bit=0, src=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(AssertionError):
        # Call target under witness parameters
        # test_cross_check_conservative_order(codes=0, rep=0, rep_min_entropy_histogram_bits_per_bit=0, rep_nist90b_bits_per_bit=1, rep_shannon_bits_per_bit=0, src=0)
        pass

def test_keller_test_thermal_statistics_match_ground_truth_line_18():
    """Formal verification counterexample for test_thermal_statistics_match_ground_truth (line 18)
    Defect class: python-assertion-failure
    Witness: [src=0, win=0, win_samples_size=1]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(AssertionError):
        # Call target under witness parameters
        # test_thermal_statistics_match_ground_truth(src=0, win=0, win_samples_size=1)
        pass

def test_keller_test_thermal_statistics_match_ground_truth_line_20():
    """Formal verification counterexample for test_thermal_statistics_match_ground_truth (line 20)
    Defect class: python-assertion-failure
    Witness: [src=0, win=0, win_samples_size=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(AssertionError):
        # Call target under witness parameters
        # test_thermal_statistics_match_ground_truth(src=0, win=0, win_samples_size=0)
        pass

def test_keller_test_thermal_entropy_ground_truth_is_positive_and_finite_line_26():
    """Formal verification counterexample for test_thermal_entropy_ground_truth_is_positive_and_finite (line 26)
    Defect class: python-assertion-failure
    Witness: [h=0, src=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(AssertionError):
        # Call target under witness parameters
        # test_thermal_entropy_ground_truth_is_positive_and_finite(h=0, src=0)
        pass

def test_keller_test_rf_is_band_limited_line_46():
    """Formal verification counterexample for test_rf_is_band_limited (line 46)
    Defect class: python-zero-division
    Witness: [freqs=0, src=0, src_sample_rate=0, win=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(ZeroDivisionError):
        # Call target under witness parameters
        # test_rf_is_band_limited(freqs=0, src=0, src_sample_rate=0, win=0)
        pass

def test_keller_test_rf_is_band_limited_line_48():
    """Formal verification counterexample for test_rf_is_band_limited (line 48)
    Defect class: python-index-out-of-bounds
    Witness: [freqs=0, in_band=0, len_psd=0, psd=2, src=0, src_bandwidth=0, win=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # test_rf_is_band_limited(freqs=0, in_band=0, len_psd=0, psd=2, src=0, src_bandwidth=0, win=0)
        pass

def test_keller_test_rf_is_band_limited_line_49():
    """Formal verification counterexample for test_rf_is_band_limited (line 49)
    Defect class: python-index-out-of-bounds
    Witness: [freqs=0, in_band=0, len_psd=0, out_band=0, psd=2, src=0, src_bandwidth=0, win=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # test_rf_is_band_limited(freqs=0, in_band=0, len_psd=0, out_band=0, psd=2, src=0, src_bandwidth=0, win=0)
        pass

def test_keller_test_rf_is_band_limited_line_50():
    """Formal verification counterexample for test_rf_is_band_limited (line 50)
    Defect class: python-assertion-failure
    Witness: [freqs=0, in_band=0, out_band=0, psd=2, src=0, win=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(AssertionError):
        # Call target under witness parameters
        # test_rf_is_band_limited(freqs=0, in_band=0, out_band=0, psd=2, src=0, win=0)
        pass

def test_keller_test_seismic_events_present_line_56():
    """Formal verification counterexample for test_seismic_events_present (line 56)
    Defect class: python-assertion-failure
    Witness: [src=0, win=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(AssertionError):
        # Call target under witness parameters
        # test_seismic_events_present(src=0, win=0)
        pass

def test_keller_test_quantize_to_bytes_shape_and_range_line_62():
    """Formal verification counterexample for test_quantize_to_bytes_shape_and_range (line 62)
    Defect class: python-assertion-failure
    Witness: [codes=0, codes_shape=1, samples=0, samples_shape=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(AssertionError):
        # Call target under witness parameters
        # test_quantize_to_bytes_shape_and_range(codes=0, codes_shape=1, samples=0, samples_shape=0)
        pass

def test_keller_test_quantize_to_bytes_shape_and_range_line_63():
    """Formal verification counterexample for test_quantize_to_bytes_shape_and_range (line 63)
    Defect class: python-assertion-failure
    Witness: [codes=0, codes_dtype=1, codes_shape=0, np_uint8=0, samples=0, samples_shape=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(AssertionError):
        # Call target under witness parameters
        # test_quantize_to_bytes_shape_and_range(codes=0, codes_dtype=1, codes_shape=0, np_uint8=0, samples=0, samples_shape=0)
        pass

def test_keller_test_quantize_to_bytes_shape_and_range_line_65():
    """Formal verification counterexample for test_quantize_to_bytes_shape_and_range (line 65)
    Defect class: python-assertion-failure
    Witness: [codes=0, codes_dtype=0, codes_shape=0, np_uint8=0, samples=0, samples_shape=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(AssertionError):
        # Call target under witness parameters
        # test_quantize_to_bytes_shape_and_range(codes=0, codes_dtype=0, codes_shape=0, np_uint8=0, samples=0, samples_shape=0)
        pass

def test_keller_test_all_sources_read_window_line_76():
    """Formal verification counterexample for test_all_sources_read_window (line 76)
    Defect class: python-assertion-failure
    Witness: [src=0, win=0, win_samples_size=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(AssertionError):
        # Call target under witness parameters
        # test_all_sources_read_window(src=0, win=0, win_samples_size=0)
        pass

def test_keller_test_all_sources_read_window_line_77():
    """Formal verification counterexample for test_all_sources_read_window (line 77)
    Defect class: python-assertion-failure
    Witness: [src=0, src_name=0, win=0, win_samples_size=1, win_source_name=1]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(AssertionError):
        # Call target under witness parameters
        # test_all_sources_read_window(src=0, src_name=0, win=0, win_samples_size=1, win_source_name=1)
        pass

def test_keller_test_certificate_serialization_round_trip_line_42():
    """Formal verification counterexample for test_certificate_serialization_round_trip (line 42)
    Defect class: python-assertion-failure
    Witness: [cert=0, cert_version=0, d=0, restored=0, restored_version=1]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(AssertionError):
        # Call target under witness parameters
        # test_certificate_serialization_round_trip(cert=0, cert_version=0, d=0, restored=0, restored_version=1)
        pass

def test_keller_test_certificate_serialization_round_trip_line_43():
    """Formal verification counterexample for test_certificate_serialization_round_trip (line 43)
    Defect class: python-assertion-failure
    Witness: [cert=0, cert_capture=0, cert_version=0, d=0, restored=0, restored_capture=1, restored_version=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(AssertionError):
        # Call target under witness parameters
        # test_certificate_serialization_round_trip(cert=0, cert_capture=0, cert_version=0, d=0, restored=0, restored_capture=1, restored_version=0)
        pass

def test_keller_test_certificate_serialization_round_trip_line_44():
    """Formal verification counterexample for test_certificate_serialization_round_trip (line 44)
    Defect class: python-assertion-failure
    Witness: [cert=0, cert_capture=0, cert_nullifier=0, cert_version=0, d=0, restored=0, restored_capture=0, restored_nullifier=1, restored_version=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(AssertionError):
        # Call target under witness parameters
        # test_certificate_serialization_round_trip(cert=0, cert_capture=0, cert_nullifier=0, cert_version=0, d=0, restored=0, restored_capture=0, restored_nullifier=1, restored_version=0)
        pass

def test_keller_test_certificate_serialization_round_trip_line_45():
    """Formal verification counterexample for test_certificate_serialization_round_trip (line 45)
    Defect class: python-assertion-failure
    Witness: [cert=0, cert_capture=0, cert_nullifier=0, cert_version=0, cert_window_id=0, d=0, restored=0, restored_capture=0, restored_nullifier=0, restored_version=0, restored_window_id=1]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(AssertionError):
        # Call target under witness parameters
        # test_certificate_serialization_round_trip(cert=0, cert_capture=0, cert_nullifier=0, cert_version=0, cert_window_id=0, d=0, restored=0, restored_capture=0, restored_nullifier=0, restored_version=0, restored_window_id=1)
        pass

def test_keller_test_certificate_serialization_round_trip_line_46():
    """Formal verification counterexample for test_certificate_serialization_round_trip (line 46)
    Defect class: python-assertion-failure
    Witness: [cert=0, cert_capture=0, cert_node_id=0, cert_nullifier=0, cert_version=0, cert_window_id=0, d=0, restored=0, restored_capture=0, restored_node_id=1, restored_nullifier=0, restored_version=0, restored_window_id=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(AssertionError):
        # Call target under witness parameters
        # test_certificate_serialization_round_trip(cert=0, cert_capture=0, cert_node_id=0, cert_nullifier=0, cert_version=0, cert_window_id=0, d=0, restored=0, restored_capture=0, restored_node_id=1, restored_nullifier=0, restored_version=0, restored_window_id=0)
        pass

def test_keller_test_certificate_serialization_round_trip_line_47():
    """Formal verification counterexample for test_certificate_serialization_round_trip (line 47)
    Defect class: python-assertion-failure
    Witness: [cert=0, cert_capture=0, cert_hardening=0, cert_node_id=0, cert_nullifier=0, cert_version=0, cert_window_id=0, d=0, restored=0, restored_capture=0, restored_hardening=1, restored_node_id=0, restored_nullifier=0, restored_version=0, restored_window_id=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(AssertionError):
        # Call target under witness parameters
        # test_certificate_serialization_round_trip(cert=0, cert_capture=0, cert_hardening=0, cert_node_id=0, cert_nullifier=0, cert_version=0, cert_window_id=0, d=0, restored=0, restored_capture=0, restored_hardening=1, restored_node_id=0, restored_nullifier=0, restored_version=0, restored_window_id=0)
        pass

def test_keller_test_certificate_serialization_round_trip_line_48():
    """Formal verification counterexample for test_certificate_serialization_round_trip (line 48)
    Defect class: python-assertion-failure
    Witness: [cert=0, cert_capture=0, cert_hardening=0, cert_mac_b64=0, cert_node_id=0, cert_nullifier=0, cert_version=0, cert_window_id=0, d=0, restored=0, restored_capture=0, restored_hardening=0, restored_mac_b64=1, restored_node_id=0, restored_nullifier=0, restored_version=0, restored_window_id=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(AssertionError):
        # Call target under witness parameters
        # test_certificate_serialization_round_trip(cert=0, cert_capture=0, cert_hardening=0, cert_mac_b64=0, cert_node_id=0, cert_nullifier=0, cert_version=0, cert_window_id=0, d=0, restored=0, restored_capture=0, restored_hardening=0, restored_mac_b64=1, restored_node_id=0, restored_nullifier=0, restored_version=0, restored_window_id=0)
        pass

def test_keller_test_certificate_body_is_canonical_line_56():
    """Formal verification counterexample for test_certificate_body_is_canonical (line 56)
    Defect class: python-assertion-failure
    Witness: [cert1=0, cert1_body_bytes=1, cert2=0, cert2_body_bytes=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(AssertionError):
        # Call target under witness parameters
        # test_certificate_body_is_canonical(cert1=0, cert1_body_bytes=1, cert2=0, cert2_body_bytes=0)
        pass

def test_keller_test_tamper_detection_each_field_line_69():
    """Formal verification counterexample for test_tamper_detection_each_field (line 69)
    Defect class: python-index-out-of-bounds
    Witness: [base=0, cert=0, d=0, key=-1, len_d=0, variations=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # test_tamper_detection_each_field(base=0, cert=0, d=0, key=-1, len_d=0, variations=0)
        pass

def test_keller_test_tamper_detection_each_field_line_74():
    """Formal verification counterexample for test_tamper_detection_each_field (line 74)
    Defect class: python-index-out-of-bounds
    Witness: [base=0, cert=0, d=0, len_d_0=0, variations=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # test_tamper_detection_each_field(base=0, cert=0, d=0, len_d_0=0, variations=0)
        pass

def test_keller_test_tamper_detection_each_field_line_74():
    """Formal verification counterexample for test_tamper_detection_each_field (line 74)
    Defect class: python-index-out-of-bounds
    Witness: [base=0, cert=0, d=0, len_d=0, variations=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # test_tamper_detection_each_field(base=0, cert=0, d=0, len_d=0, variations=0)
        pass

def test_keller_test_entropy_digest_deterministic_and_sensitive_line_100():
    """Formal verification counterexample for test_entropy_digest_deterministic_and_sensitive (line 100)
    Defect class: python-assertion-failure
    Witness: [a=0, b=0, c=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(AssertionError):
        # Call target under witness parameters
        # test_entropy_digest_deterministic_and_sensitive(a=0, b=0, c=0)
        pass

def test_keller_test_split_and_reconstruct_round_trip_line_27():
    """Formal verification counterexample for test_split_and_reconstruct_round_trip (line 27)
    Defect class: python-index-out-of-bounds
    Witness: [i=-1, len_shares=0, rng=0, secret=123456789, sel=0, shares=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # test_split_and_reconstruct_round_trip(i=-1, len_shares=0, rng=0, secret=123456789, sel=0, shares=0)
        pass

def test_keller_test_split_and_reconstruct_round_trip_line_28():
    """Formal verification counterexample for test_split_and_reconstruct_round_trip (line 28)
    Defect class: python-assertion-failure
    Witness: [rng=0, secret=123456789, sel=0, shares=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(AssertionError):
        # Call target under witness parameters
        # test_split_and_reconstruct_round_trip(rng=0, secret=123456789, sel=0, shares=0)
        pass

def test_keller_test_shares_are_additively_homomorphic_line_42():
    """Formal verification counterexample for test_shares_are_additively_homomorphic (line 42)
    Defect class: python-zero-division
    Witness: [_PRIME=0, diffs=0, rng=0, sa=0, sb=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(ZeroDivisionError):
        # Call target under witness parameters
        # test_shares_are_additively_homomorphic(_PRIME=0, diffs=0, rng=0, sa=0, sb=0)
        pass

def test_keller_test_shares_are_additively_homomorphic_line_42():
    """Formal verification counterexample for test_shares_are_additively_homomorphic (line 42)
    Defect class: python-index-out-of-bounds
    Witness: [diffs=0, i=-1, len_sa=0, rng=0, sa=0, sb=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # test_shares_are_additively_homomorphic(diffs=0, i=-1, len_sa=0, rng=0, sa=0, sb=0)
        pass

def test_keller_test_shares_are_additively_homomorphic_line_42():
    """Formal verification counterexample for test_shares_are_additively_homomorphic (line 42)
    Defect class: python-index-out-of-bounds
    Witness: [diffs=0, i=-1, len_sb=0, rng=0, sa=0, sb=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # test_shares_are_additively_homomorphic(diffs=0, i=-1, len_sb=0, rng=0, sa=0, sb=0)
        pass

def test_keller_test_shares_are_additively_homomorphic_line_43():
    """Formal verification counterexample for test_shares_are_additively_homomorphic (line 43)
    Defect class: python-assertion-failure
    Witness: [_PRIME=2, a=1, b=0, diffs=0, div0=(x!1, mod0=(x!1, rng=0, sa=0, sb=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(AssertionError):
        # Call target under witness parameters
        # test_shares_are_additively_homomorphic(_PRIME=2, a=1, b=0, diffs=0, div0=(x!1, mod0=(x!1, rng=0, sa=0, sb=0)
        pass

def test_keller_test_shares_are_additively_homomorphic_line_43():
    """Formal verification counterexample for test_shares_are_additively_homomorphic (line 43)
    Defect class: python-zero-division
    Witness: [_PRIME=0, a=21238, b=0, diffs=0, mod0=(x!1, rng=0, sa=0, sb=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(ZeroDivisionError):
        # Call target under witness parameters
        # test_shares_are_additively_homomorphic(_PRIME=0, a=21238, b=0, diffs=0, mod0=(x!1, rng=0, sa=0, sb=0)
        pass

def test_keller_test_no_cross_no_matches_line_74():
    """Formal verification counterexample for test_no_cross_no_matches (line 74)
    Defect class: python-assertion-failure
    Witness: [bids=0, offers=0, out=0, out_matches=1]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(AssertionError):
        # Call target under witness parameters
        # test_no_cross_no_matches(bids=0, offers=0, out=0, out_matches=1)
        pass

def test_keller_test_match_quantity_is_min_line_81():
    """Formal verification counterexample for test_match_quantity_is_min (line 81)
    Defect class: python-assertion-failure
    Witness: [bids=0, len_out_matches=0, offers=0, out=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(AssertionError):
        # Call target under witness parameters
        # test_match_quantity_is_min(bids=0, len_out_matches=0, offers=0, out=0)
        pass

def test_keller_test_match_quantity_is_min_line_82():
    """Formal verification counterexample for test_match_quantity_is_min (line 82)
    Defect class: python-assertion-failure
    Witness: [bids=0, len_out_matches=1, offers=0, out=0, out_matches_0_quantity=4]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(AssertionError):
        # Call target under witness parameters
        # test_match_quantity_is_min(bids=0, len_out_matches=1, offers=0, out=0, out_matches_0_quantity=4)
        pass

def test_keller_test_match_quantity_is_min_line_83():
    """Formal verification counterexample for test_match_quantity_is_min (line 83)
    Defect class: python-assertion-failure
    Witness: [bids=0, len_out_matches=1, offers=0, out=0, out_matches_0_price=101, out_matches_0_quantity=3]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(AssertionError):
        # Call target under witness parameters
        # test_match_quantity_is_min(bids=0, len_out_matches=1, offers=0, out=0, out_matches_0_price=101, out_matches_0_quantity=3)
        pass

def test_keller_test_agents_never_see_prices_line_116():
    """Formal verification counterexample for test_agents_never_see_prices (line 116)
    Defect class: python-assertion-failure
    Witness: [_PRIME=0, bids=0, offers=0, rng=0, share=-1, shares=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(AssertionError):
        # Call target under witness parameters
        # test_agents_never_see_prices(_PRIME=0, bids=0, offers=0, rng=0, share=-1, shares=0)
        pass

def test_keller__render_line_30():
    """Formal verification counterexample for _render (line 30)
    Defect class: python-zero-division
    Witness: [out=0, tmp_path=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(ZeroDivisionError):
        # Call target under witness parameters
        # _render(out=0, tmp_path=0)
        pass

def test_keller_test_topology_is_deterministic_line_25():
    """Formal verification counterexample for test_topology_is_deterministic (line 25)
    Defect class: python-assertion-failure
    Witness: [t1=0, t2=0, t3=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(AssertionError):
        # Call target under witness parameters
        # test_topology_is_deterministic(t1=0, t2=0, t3=0)
        pass

def test_keller_test_direct_delivery_when_link_up_line_38():
    """Formal verification counterexample for test_direct_delivery_when_link_up (line 38)
    Defect class: python-index-out-of-bounds
    Witness: [bundle=0, len_res_deliveries=0, rec=0, res=0, res_deliveries_0=0, topo=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # test_direct_delivery_when_link_up(bundle=0, len_res_deliveries=0, rec=0, res=0, res_deliveries_0=0, topo=0)
        pass

def test_keller_test_direct_delivery_when_link_up_line_39():
    """Formal verification counterexample for test_direct_delivery_when_link_up (line 39)
    Defect class: python-assertion-failure
    Witness: [bundle=0, rec=0, rec_hops=2, res=0, res_deliveries_0=0, topo=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(AssertionError):
        # Call target under witness parameters
        # test_direct_delivery_when_link_up(bundle=0, rec=0, rec_hops=2, res=0, res_deliveries_0=0, topo=0)
        pass

def test_keller_test_direct_delivery_when_link_up_line_40():
    """Formal verification counterexample for test_direct_delivery_when_link_up (line 40)
    Defect class: python-assertion-failure
    Witness: [bundle=0, rec=0, rec_hops=1, rec_path=1, res=0, res_deliveries_0=0, topo=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(AssertionError):
        # Call target under witness parameters
        # test_direct_delivery_when_link_up(bundle=0, rec=0, rec_hops=1, rec_path=1, res=0, res_deliveries_0=0, topo=0)
        pass

def test_keller_test_undelivered_when_never_connected_line_47():
    """Formal verification counterexample for test_undelivered_when_never_connected (line 47)
    Defect class: python-assertion-failure
    Witness: [bundle=0, res=0, res_deliveries=1, topo=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(AssertionError):
        # Call target under witness parameters
        # test_undelivered_when_never_connected(bundle=0, res=0, res_deliveries=1, topo=0)
        pass

def test_keller_test_multihop_store_and_forward_line_59():
    """Formal verification counterexample for test_multihop_store_and_forward (line 59)
    Defect class: python-index-out-of-bounds
    Witness: [bundle=0, len_res_deliveries=0, rec=0, res=0, res_deliveries_0=0, topo=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # test_multihop_store_and_forward(bundle=0, len_res_deliveries=0, rec=0, res=0, res_deliveries_0=0, topo=0)
        pass

def test_keller_test_multihop_store_and_forward_line_60():
    """Formal verification counterexample for test_multihop_store_and_forward (line 60)
    Defect class: python-assertion-failure
    Witness: [bundle=0, rec=0, rec_hops=3, res=0, res_deliveries_0=0, topo=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(AssertionError):
        # Call target under witness parameters
        # test_multihop_store_and_forward(bundle=0, rec=0, rec_hops=3, res=0, res_deliveries_0=0, topo=0)
        pass

def test_keller_test_multihop_store_and_forward_line_61():
    """Formal verification counterexample for test_multihop_store_and_forward (line 61)
    Defect class: python-assertion-failure
    Witness: [bundle=0, rec=0, rec_hops=2, rec_path=1, res=0, res_deliveries_0=0, topo=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(AssertionError):
        # Call target under witness parameters
        # test_multihop_store_and_forward(bundle=0, rec=0, rec_hops=2, rec_path=1, res=0, res_deliveries_0=0, topo=0)
        pass

def test_keller_test_no_duplicate_deliveries_line_91():
    """Formal verification counterexample for test_no_duplicate_deliveries (line 91)
    Defect class: python-assertion-failure
    Witness: [bundle=0, len_res_deliveries=0, res=0, topo=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(AssertionError):
        # Call target under witness parameters
        # test_no_duplicate_deliveries(bundle=0, len_res_deliveries=0, res=0, topo=0)
        pass

def test_keller_test_ttl_expiry_drops_bundles_line_98():
    """Formal verification counterexample for test_ttl_expiry_drops_bundles (line 98)
    Defect class: python-assertion-failure
    Witness: [bundle=0, res=0, res_deliveries=1, topo=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(AssertionError):
        # Call target under witness parameters
        # test_ttl_expiry_drops_bundles(bundle=0, res=0, res_deliveries=1, topo=0)
        pass

def test_keller_test_ttl_expiry_drops_bundles_line_99():
    """Formal verification counterexample for test_ttl_expiry_drops_bundles (line 99)
    Defect class: python-assertion-failure
    Witness: [bundle=0, res=0, res_deliveries=0, res_dropped=0, topo=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(AssertionError):
        # Call target under witness parameters
        # test_ttl_expiry_drops_bundles(bundle=0, res=0, res_deliveries=0, res_dropped=0, topo=0)
        pass

def test_keller_test_encode_decode_round_trip_line_16():
    """Formal verification counterexample for test_encode_decode_round_trip (line 16)
    Defect class: python-assertion-failure
    Witness: [data=0, len_shards=0, n=1, shards=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(AssertionError):
        # Call target under witness parameters
        # test_encode_decode_round_trip(data=0, len_shards=0, n=1, shards=0)
        pass

def test_keller_test_systematic_shards_are_raw_blocks_line_39():
    """Formal verification counterexample for test_systematic_shards_are_raw_blocks (line 39)
    Defect class: python-assertion-failure
    Witness: [block_len=20, data=0, len_shards_0_data=0, shards=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(AssertionError):
        # Call target under witness parameters
        # test_systematic_shards_are_raw_blocks(block_len=20, data=0, len_shards_0_data=0, shards=0)
        pass

def test_keller_test_systematic_shards_are_raw_blocks_line_40():
    """Formal verification counterexample for test_systematic_shards_are_raw_blocks (line 40)
    Defect class: python-assertion-failure
    Witness: [block_len=20, data=0, data_0=0, len_shards_0_data=20, shards=0, shards_0_data_0=1]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(AssertionError):
        # Call target under witness parameters
        # test_systematic_shards_are_raw_blocks(block_len=20, data=0, data_0=0, len_shards_0_data=20, shards=0, shards_0_data_0=1)
        pass

def test_keller_test_systematic_shards_are_raw_blocks_line_41():
    """Formal verification counterexample for test_systematic_shards_are_raw_blocks (line 41)
    Defect class: python-assertion-failure
    Witness: [block_len=20, data=0, data_0=0, len_shards_0_data=20, shards=0, shards_0_data_0=0, shards_1_data_0=1]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(AssertionError):
        # Call target under witness parameters
        # test_systematic_shards_are_raw_blocks(block_len=20, data=0, data_0=0, len_shards_0_data=20, shards=0, shards_0_data_0=0, shards_1_data_0=1)
        pass

def test_keller_test_systematic_shards_are_raw_blocks_line_42():
    """Formal verification counterexample for test_systematic_shards_are_raw_blocks (line 42)
    Defect class: python-assertion-failure
    Witness: [block_len=20, data=0, data_0=0, len_shards_0_data=20, shards=0, shards_0_data_0=0, shards_1_data_0=0, shards_2_data_0=1]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(AssertionError):
        # Call target under witness parameters
        # test_systematic_shards_are_raw_blocks(block_len=20, data=0, data_0=0, len_shards_0_data=20, shards=0, shards_0_data_0=0, shards_1_data_0=0, shards_2_data_0=1)
        pass

def test_keller_test_systematic_shards_are_raw_blocks_line_44():
    """Formal verification counterexample for test_systematic_shards_are_raw_blocks (line 44)
    Defect class: python-assertion-failure
    Witness: [block_len=20, data=0, data_0=0, len_shards_0_data=20, shards=0, shards_0_data=0, shards_0_data_0=0, shards_1_data_0=0, shards_2_data_0=0, shards_3_data=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(AssertionError):
        # Call target under witness parameters
        # test_systematic_shards_are_raw_blocks(block_len=20, data=0, data_0=0, len_shards_0_data=20, shards=0, shards_0_data=0, shards_0_data_0=0, shards_1_data_0=0, shards_2_data_0=0, shards_3_data=0)
        pass

def test_keller_test_systematic_shards_are_raw_blocks_line_45():
    """Formal verification counterexample for test_systematic_shards_are_raw_blocks (line 45)
    Defect class: python-assertion-failure
    Witness: [block_len=20, data=0, data_0=0, len_shards_0_data=20, shards=0, shards_0_data=0, shards_0_data_0=0, shards_1_data=0, shards_1_data_0=0, shards_2_data_0=0, shards_3_data=1, shards_4_data=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(AssertionError):
        # Call target under witness parameters
        # test_systematic_shards_are_raw_blocks(block_len=20, data=0, data_0=0, len_shards_0_data=20, shards=0, shards_0_data=0, shards_0_data_0=0, shards_1_data=0, shards_1_data_0=0, shards_2_data_0=0, shards_3_data=1, shards_4_data=0)
        pass

def test_keller_test_corrupt_shard_fails_or_recovers_line_60():
    """Formal verification counterexample for test_corrupt_shard_fails_or_recovers (line 60)
    Defect class: python-index-out-of-bounds
    Witness: [bad=0, data=0, len_shards=0, shards=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # test_corrupt_shard_fails_or_recovers(bad=0, data=0, len_shards=0, shards=0)
        pass

def test_keller_test_corrupt_shard_fails_or_recovers_line_61():
    """Formal verification counterexample for test_corrupt_shard_fails_or_recovers (line 61)
    Defect class: python-index-out-of-bounds
    Witness: [bad=0, data=0, len_bad=0, shards=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # test_corrupt_shard_fails_or_recovers(bad=0, data=0, len_bad=0, shards=0)
        pass

def test_keller_test_corrupt_shard_fails_or_recovers_line_62():
    """Formal verification counterexample for test_corrupt_shard_fails_or_recovers (line 62)
    Defect class: python-index-out-of-bounds
    Witness: [bad=0, data=0, good=0, i=-1, len_shards=0, shards=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # test_corrupt_shard_fails_or_recovers(bad=0, data=0, good=0, i=-1, len_shards=0, shards=0)
        pass

def test_keller_test_corrupt_shard_fails_or_recovers_line_66():
    """Formal verification counterexample for test_corrupt_shard_fails_or_recovers (line 66)
    Defect class: python-index-out-of-bounds
    Witness: [bad=0, data=0, good=0, good_0=0, len_good=0, shards=0, with_bad=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # test_corrupt_shard_fails_or_recovers(bad=0, data=0, good=0, good_0=0, len_good=0, shards=0, with_bad=0)
        pass

def test_keller_test_corrupt_shard_fails_or_recovers_line_68():
    """Formal verification counterexample for test_corrupt_shard_fails_or_recovers (line 68)
    Defect class: python-assertion-failure
    Witness: [bad=0, data=0, good=0, good_0=0, result=0, shards=0, with_bad=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(AssertionError):
        # Call target under witness parameters
        # test_corrupt_shard_fails_or_recovers(bad=0, data=0, good=0, good_0=0, result=0, shards=0, with_bad=0)
        pass

def test_keller_test_decode_requires_matching_length_headers_line_99():
    """Formal verification counterexample for test_decode_requires_matching_length_headers (line 99)
    Defect class: python-index-out-of-bounds
    Witness: [bad=0, data=0, len_tampered=0, shards=0, tampered=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # test_decode_requires_matching_length_headers(bad=0, data=0, len_tampered=0, shards=0, tampered=0)
        pass

def test_keller_test_decode_requires_matching_length_headers_line_100():
    """Formal verification counterexample for test_decode_requires_matching_length_headers (line 100)
    Defect class: python-index-out-of-bounds
    Witness: [bad=0, data=0, len_bad=0, shards=0, tampered=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # test_decode_requires_matching_length_headers(bad=0, data=0, len_bad=0, shards=0, tampered=0)
        pass

def test_keller_test_gf_mul_known_values_line_29():
    """Formal verification counterexample for test_gf_mul_known_values (line 29)
    Defect class: python-assertion-failure
    Witness: []"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(AssertionError):
        # Call target under witness parameters
        # test_gf_mul_known_values()
        pass

def test_keller_test_gf_pow_is_cycle_255_line_62():
    """Formal verification counterexample for test_gf_pow_is_cycle_255 (line 62)
    Defect class: python-assertion-failure
    Witness: [a=0, rng=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(AssertionError):
        # Call target under witness parameters
        # test_gf_pow_is_cycle_255(a=0, rng=0)
        pass

def test_keller_test_gf_poly_eval_constant_poly_line_76():
    """Formal verification counterexample for test_gf_poly_eval_constant_poly (line 76)
    Defect class: python-assertion-failure
    Witness: [coeffs=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(AssertionError):
        # Call target under witness parameters
        # test_gf_poly_eval_constant_poly(coeffs=0)
        pass

def test_keller_test_extract_seed_deterministic_and_sensitive_line_89():
    """Formal verification counterexample for test_extract_seed_deterministic_and_sensitive (line 89)
    Defect class: python-assertion-failure
    Witness: [a=0, b=0, c=0, raw=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(AssertionError):
        # Call target under witness parameters
        # test_extract_seed_deterministic_and_sensitive(a=0, b=0, c=0, raw=0)
        pass

def test_keller_test_polynomial_embed_pads_and_truncates_line_96():
    """Formal verification counterexample for test_polynomial_embed_pads_and_truncates (line 96)
    Defect class: python-assertion-failure
    Witness: [coeffs=0, len_long=0, long=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(AssertionError):
        # Call target under witness parameters
        # test_polynomial_embed_pads_and_truncates(coeffs=0, len_long=0, long=0)
        pass

def test_keller_test_verify_rejects_corrupted_eval_or_raw_line_122():
    """Formal verification counterexample for test_verify_rejects_corrupted_eval_or_raw (line 122)
    Defect class: python-index-out-of-bounds
    Witness: [artifact=0, bad_eval=0, len_bad_eval=0, raw=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # test_verify_rejects_corrupted_eval_or_raw(artifact=0, bad_eval=0, len_bad_eval=0, raw=0)
        pass

def test_keller_test_artifact_serialization_round_trip_line_141():
    """Formal verification counterexample for test_artifact_serialization_round_trip (line 141)
    Defect class: python-assertion-failure
    Witness: [artifact=0, artifact_params=0, d=0, raw=0, restored=0, restored_params=1]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(AssertionError):
        # Call target under witness parameters
        # test_artifact_serialization_round_trip(artifact=0, artifact_params=0, d=0, raw=0, restored=0, restored_params=1)
        pass

def test_keller_test_artifact_serialization_round_trip_line_143():
    """Formal verification counterexample for test_artifact_serialization_round_trip (line 143)
    Defect class: python-assertion-failure
    Witness: [artifact=0, artifact_mac=0, artifact_params=0, d=0, raw=0, restored=0, restored_mac=1, restored_params=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(AssertionError):
        # Call target under witness parameters
        # test_artifact_serialization_round_trip(artifact=0, artifact_mac=0, artifact_params=0, d=0, raw=0, restored=0, restored_mac=1, restored_params=0)
        pass

def test_keller_test_harden_different_raw_gives_different_artifact_line_150():
    """Formal verification counterexample for test_harden_different_raw_gives_different_artifact (line 150)
    Defect class: python-assertion-failure
    Witness: [a=0, b=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(AssertionError):
        # Call target under witness parameters
        # test_harden_different_raw_gives_different_artifact(a=0, b=0)
        pass

def test_keller_test_pq_func_known_values_line_59():
    """Formal verification counterexample for test_pq_func_known_values (line 59)
    Defect class: python-assertion-failure
    Witness: []"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(AssertionError):
        # Call target under witness parameters
        # test_pq_func_known_values()
        pass

def test_keller_test_pq_func_numerically_stable_near_one_line_65():
    """Formal verification counterexample for test_pq_func_numerically_stable_near_one (line 65)
    Defect class: python-assertion-failure
    Witness: []"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(AssertionError):
        # Call target under witness parameters
        # test_pq_func_numerically_stable_near_one()
        pass

def test_keller_test_pfunc_and_search_line_77():
    """Formal verification counterexample for test_pfunc_and_search (line 77)
    Defect class: python-assertion-failure
    Witness: []"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(AssertionError):
        # Call target under witness parameters
        # test_pfunc_and_search()
        pass

def test_keller_test_random_bytes_sanity_line_99():
    """Formal verification counterexample for test_random_bytes_sanity (line 99)
    Defect class: python-assertion-failure
    Witness: [h=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(AssertionError):
        # Call target under witness parameters
        # test_random_bytes_sanity(h=0)
        pass

def test_keller_test_random_bits_sanity_line_105():
    """Formal verification counterexample for test_random_bits_sanity (line 105)
    Defect class: python-assertion-failure
    Witness: [h=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(AssertionError):
        # Call target under witness parameters
        # test_random_bits_sanity(h=0)
        pass

def test_keller_test_estimate_min_entropy_small_random_bytes_line_146():
    """Formal verification counterexample for test_estimate_min_entropy_small_random_bytes (line 146)
    Defect class: python-assertion-failure
    Witness: [res=0, res_0=9]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(AssertionError):
        # Call target under witness parameters
        # test_estimate_min_entropy_small_random_bytes(res=0, res_0=9)
        pass

def test_keller_test_estimate_min_entropy_small_random_bytes_line_147():
    """Formal verification counterexample for test_estimate_min_entropy_small_random_bytes (line 147)
    Defect class: python-assertion-failure
    Witness: [res=0, res_0=8]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(AssertionError):
        # Call target under witness parameters
        # test_estimate_min_entropy_small_random_bytes(res=0, res_0=8)
        pass

def test_keller_test_estimate_min_entropy_accepts_bytes_and_array_line_158():
    """Formal verification counterexample for test_estimate_min_entropy_accepts_bytes_and_array (line 158)
    Defect class: python-assertion-failure
    Witness: [arr=0, res=0, res_0=0, same=0, same_0=1]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(AssertionError):
        # Call target under witness parameters
        # test_estimate_min_entropy_accepts_bytes_and_array(arr=0, res=0, res_0=0, same=0, same_0=1)
        pass

def test_keller_test_estimate_min_entropy_respects_limit_line_164():
    """Formal verification counterexample for test_estimate_min_entropy_respects_limit (line 164)
    Defect class: python-assertion-failure
    Witness: [res=0, res_0=10001]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(AssertionError):
        # Call target under witness parameters
        # test_estimate_min_entropy_respects_limit(res=0, res_0=10001)
        pass

def test_keller_test_random_bytes_near_one_bit_per_bit_line_185():
    """Formal verification counterexample for test_random_bytes_near_one_bit_per_bit (line 185)
    Defect class: python-assertion-failure
    Witness: [h=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(AssertionError):
        # Call target under witness parameters
        # test_random_bytes_near_one_bit_per_bit(h=0)
        pass

def test_keller_test_random_bits_near_one_bit_line_199():
    """Formal verification counterexample for test_random_bits_near_one_bit (line 199)
    Defect class: python-assertion-failure
    Witness: [BINARY_SLOW_FLOOR_est___name__=1, h=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(AssertionError):
        # Call target under witness parameters
        # test_random_bits_near_one_bit(BINARY_SLOW_FLOOR_est___name__=1, h=0)
        pass

def test_keller_test_estimate_min_entropy_random_bytes_line_206():
    """Formal verification counterexample for test_estimate_min_entropy_random_bytes (line 206)
    Defect class: python-assertion-failure
    Witness: [res=0, res_0=9]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(AssertionError):
        # Call target under witness parameters
        # test_estimate_min_entropy_random_bytes(res=0, res_0=9)
        pass

def test_keller_test_estimate_min_entropy_random_bytes_line_207():
    """Formal verification counterexample for test_estimate_min_entropy_random_bytes (line 207)
    Defect class: python-assertion-failure
    Witness: [res=0, res_0=8]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(AssertionError):
        # Call target under witness parameters
        # test_estimate_min_entropy_random_bytes(res=0, res_0=8)
        pass

def test_keller_test_estimate_min_entropy_random_bits_line_225():
    """Formal verification counterexample for test_estimate_min_entropy_random_bits (line 225)
    Defect class: python-assertion-failure
    Witness: [res=0, res_0=2]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(AssertionError):
        # Call target under witness parameters
        # test_estimate_min_entropy_random_bits(res=0, res_0=2)
        pass

def test_keller_test_estimate_min_entropy_random_bits_line_227():
    """Formal verification counterexample for test_estimate_min_entropy_random_bits (line 227)
    Defect class: python-assertion-failure
    Witness: [res=0, res_0=1]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(AssertionError):
        # Call target under witness parameters
        # test_estimate_min_entropy_random_bits(res=0, res_0=1)
        pass

def test_keller_test_estimate_min_entropy_constant_is_zero_line_235():
    """Formal verification counterexample for test_estimate_min_entropy_constant_is_zero (line 235)
    Defect class: python-assertion-failure
    Witness: [res=0, res_0=1]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(AssertionError):
        # Call target under witness parameters
        # test_estimate_min_entropy_constant_is_zero(res=0, res_0=1)
        pass

def test_keller_test_same_signal_same_window_deterministic_line_31():
    """Formal verification counterexample for test_same_signal_same_window_deterministic (line 31)
    Defect class: python-assertion-failure
    Witness: [key=0, len_n1=0, n1=0, n2=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(AssertionError):
        # Call target under witness parameters
        # test_same_signal_same_window_deterministic(key=0, len_n1=0, n1=0, n2=0)
        pass

def test_keller_test_same_signal_different_window_differs_line_38():
    """Formal verification counterexample for test_same_signal_different_window_differs (line 38)
    Defect class: python-assertion-failure
    Witness: [key=0, n1=0, n2=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(AssertionError):
        # Call target under witness parameters
        # test_same_signal_different_window_differs(key=0, n1=0, n2=0)
        pass

def test_keller_test_same_signal_different_node_differs_line_44():
    """Formal verification counterexample for test_same_signal_different_node_differs (line 44)
    Defect class: python-assertion-failure
    Witness: [a=0, b=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(AssertionError):
        # Call target under witness parameters
        # test_same_signal_different_node_differs(a=0, b=0)
        pass

def test_keller_test_different_signal_same_node_window_differs_line_49():
    """Formal verification counterexample for test_different_signal_same_node_window_differs (line 49)
    Defect class: python-assertion-failure
    Witness: [key=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(AssertionError):
        # Call target under witness parameters
        # test_different_signal_same_node_window_differs(key=0)
        pass

def test_keller_test_nullifier_hides_seed_line_69():
    """Formal verification counterexample for test_nullifier_hides_seed (line 69)
    Defect class: python-assertion-failure
    Witness: [key=0, n=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(AssertionError):
        # Call target under witness parameters
        # test_nullifier_hides_seed(key=0, n=0)
        pass

def test_keller_test_nullifier_key_deterministic_line_76():
    """Formal verification counterexample for test_nullifier_key_deterministic (line 76)
    Defect class: python-assertion-failure
    Witness: [a=0, a_key_material=1, b=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(AssertionError):
        # Call target under witness parameters
        # test_nullifier_key_deterministic(a=0, a_key_material=1, b=0)
        pass

def test_keller_test_settle_pays_payee_and_marks_spent_line_20():
    """Formal verification counterexample for test_settle_pays_payee_and_marks_spent (line 20)
    Defect class: python-assertion-failure
    Witness: [nul=0, s=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(AssertionError):
        # Call target under witness parameters
        # test_settle_pays_payee_and_marks_spent(nul=0, s=0)
        pass

def test_keller_test_double_settle_same_nullifier_reverted_line_31():
    """Formal verification counterexample for test_double_settle_same_nullifier_reverted (line 31)
    Defect class: python-assertion-failure
    Witness: [s=0, s_count=2]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(AssertionError):
        # Call target under witness parameters
        # test_double_settle_same_nullifier_reverted(s=0, s_count=2)
        pass

def test_keller_test_surplus_refunded_line_56():
    """Formal verification counterexample for test_surplus_refunded (line 56)
    Defect class: python-assertion-failure
    Witness: [s=0, s_balances_0=101]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(AssertionError):
        # Call target under witness parameters
        # test_surplus_refunded(s=0, s_balances_0=101)
        pass

def test_keller_test_surplus_refunded_line_57():
    """Formal verification counterexample for test_surplus_refunded (line 57)
    Defect class: python-assertion-failure
    Witness: [s=0, s_balances_0=100]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(AssertionError):
        # Call target under witness parameters
        # test_surplus_refunded(s=0, s_balances_0=100)
        pass

def test_keller__peel_all_line_31():
    """Formal verification counterexample for _peel_all (line 31)
    Defect class: python-assertion-failure
    Witness: [cur=0, final_payload=0, nodes=0, packet=0, res=0, res_next_packet=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(AssertionError):
        # Call target under witness parameters
        # _peel_all(cur=0, final_payload=0, nodes=0, packet=0, res=0, res_next_packet=0)
        pass

def test_keller__peel_all_line_34():
    """Formal verification counterexample for _peel_all (line 34)
    Defect class: python-assertion-failure
    Witness: [cur=0, final_payload=0, nodes=0, packet=0, res=0, res_next_packet=1, res_next_packet_0=0, res_next_tag=1]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(AssertionError):
        # Call target under witness parameters
        # _peel_all(cur=0, final_payload=0, nodes=0, packet=0, res=0, res_next_packet=1, res_next_packet_0=0, res_next_tag=1)
        pass

def test_keller_test_intermediate_hop_does_not_see_payload_line_55():
    """Formal verification counterexample for test_intermediate_hop_does_not_see_payload (line 55)
    Defect class: python-index-out-of-bounds
    Witness: [hop0=0, len_ROUTE=0, packet=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # test_intermediate_hop_does_not_see_payload(hop0=0, len_ROUTE=0, packet=0)
        pass

def test_keller_test_replay_rejected_line_66():
    """Formal verification counterexample for test_replay_rejected (line 66)
    Defect class: python-index-out-of-bounds
    Witness: [hop0=0, len_ROUTE=0, packet=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # test_replay_rejected(hop0=0, len_ROUTE=0, packet=0)
        pass

def test_keller_test_out_of_order_rejected_line_75():
    """Formal verification counterexample for test_out_of_order_rejected (line 75)
    Defect class: python-index-out-of-bounds
    Witness: [first=0, hop0=0, len_ROUTE=0, second=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # test_out_of_order_rejected(first=0, hop0=0, len_ROUTE=0, second=0)
        pass

def test_keller_test_tamper_detection_line_91():
    """Formal verification counterexample for test_tamper_detection (line 91)
    Defect class: python-index-out-of-bounds
    Witness: [len_packet=4, packet=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # test_tamper_detection(len_packet=4, packet=0)
        pass

def test_keller_test_tamper_detection_line_92():
    """Formal verification counterexample for test_tamper_detection (line 92)
    Defect class: python-index-out-of-bounds
    Witness: [hop0=0, len_ROUTE=0, packet=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # test_tamper_detection(hop0=0, len_ROUTE=0, packet=0)
        pass

def test_keller_test_true_quantized_gaussian_entropy_values_line_21():
    """Formal verification counterexample for test_true_quantized_gaussian_entropy_values (line 21)
    Defect class: python-assertion-failure
    Witness: [h=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(AssertionError):
        # Call target under witness parameters
        # test_true_quantized_gaussian_entropy_values(h=0)
        pass

def test_keller_test_validate_sources_thermal_ground_truth_line_63():
    """Formal verification counterexample for test_validate_sources_thermal_ground_truth (line 63)
    Defect class: python-index-out-of-bounds
    Witness: [len_report_sources=0, report=0, report_sources_0=0, s=0, src=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # test_validate_sources_thermal_ground_truth(len_report_sources=0, report=0, report_sources_0=0, s=0, src=0)
        pass

def test_keller_test_validate_sources_thermal_ground_truth_line_64():
    """Formal verification counterexample for test_validate_sources_thermal_ground_truth (line 64)
    Defect class: python-assertion-failure
    Witness: [report=0, report_sources_0=0, s=0, s_ground_truth_bits_per_byte=0, src=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(AssertionError):
        # Call target under witness parameters
        # test_validate_sources_thermal_ground_truth(report=0, report_sources_0=0, s=0, s_ground_truth_bits_per_byte=0, src=0)
        pass

def test_keller_test_validate_sources_thermal_ground_truth_line_65():
    """Formal verification counterexample for test_validate_sources_thermal_ground_truth (line 65)
    Defect class: python-assertion-failure
    Witness: [report=0, report_sources_0=0, s=0, s_ground_truth_bits_per_byte=-1, s_shannon_bits_per_byte=0, src=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(AssertionError):
        # Call target under witness parameters
        # test_validate_sources_thermal_ground_truth(report=0, report_sources_0=0, s=0, s_ground_truth_bits_per_byte=-1, s_shannon_bits_per_byte=0, src=0)
        pass
