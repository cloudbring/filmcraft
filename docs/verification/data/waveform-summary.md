
### speech  (publishable: true, contended: false, load Some(3.68)->Some(3.68), 40 runs/side)

| input | old median | new median | wall speedup [95% CI] | cpu speedup [95% CI] | verdict |
|---|---|---|---|---|---|
| speech/native(300s)/whole/w1000 | 137.083 us | 46.771 us | 2.93x [2.81, 3.05] | 2.93x [2.78, 3.03] | faster |
| speech/native(300s)/whole/w3840 | 207.875 us | 153.708 us | 1.35x [1.29, 1.43] | 1.35x [1.29, 1.44] | faster |
| speech/native(300s)/1/4/w1000 | 52.438 us | 39.312 us | 1.33x [1.21, 1.39] | 1.31x [1.21, 1.37] | faster |
| speech/native(300s)/1/16/w1000 | 31.271 us | 32.146 us | 0.97x [0.93, 1.04] | 1.00x [0.94, 1.03] | inconclusive (CI includes 1.0) |
| speech/native(300s)/30s/w1000 | 35.249 us | 32.791 us | 1.07x [1.02, 1.14] | 1.09x [1.01, 1.15] | faster |
| speech/native(300s)/1s/w1000 | 18.645 us | 22.187 us | 0.84x [0.81, 0.87] | 0.84x [0.78, 0.86] | slower |
| speech/2h-tiled(7200s)/whole/w1000 | 2.556 ms | 63.583 us | 40.20x [35.79, 41.84] | 40.04x [35.61, 41.70] | faster |
| speech/2h-tiled(7200s)/whole/w3840 | 2.667 ms | 237.417 us | 11.24x [10.75, 11.70] | 11.23x [10.69, 11.65] | faster |
| speech/2h-tiled(7200s)/1/4/w1000 | 652.542 us | 61.000 us | 10.70x [10.32, 11.08] | 10.52x [10.28, 10.92] | faster |
| speech/2h-tiled(7200s)/1/16/w1000 | 198.396 us | 52.480 us | 3.78x [3.62, 4.02] | 3.75x [3.60, 4.02] | faster |
| speech/2h-tiled(7200s)/30s/w1000 | 35.479 us | 32.500 us | 1.09x [1.04, 1.15] | 1.12x [1.04, 1.16] | faster |
| speech/2h-tiled(7200s)/1s/w1000 | 26.437 us | 32.229 us | 0.82x [0.80, 0.85] | 0.83x [0.79, 0.84] | slower |
| **geomean** | | | **2.55x [2.51, 2.59]** | 2.56x [2.50, 2.59] | |

### music  (publishable: true, contended: false, load Some(3.68)->Some(3.68), 40 runs/side)

| input | old median | new median | wall speedup [95% CI] | cpu speedup [95% CI] | verdict |
|---|---|---|---|---|---|
| music/native(240s)/whole/w1000 | 126.084 us | 58.251 us | 2.16x [2.11, 2.18] | 2.14x [2.09, 2.18] | faster |
| music/native(240s)/whole/w3840 | 224.604 us | 192.917 us | 1.16x [1.12, 1.18] | 1.17x [1.12, 1.18] | faster |
| music/native(240s)/1/4/w1000 | 58.730 us | 50.583 us | 1.16x [1.13, 1.18] | 1.16x [1.14, 1.19] | faster |
| music/native(240s)/1/16/w1000 | 41.999 us | 43.979 us | 0.95x [0.90, 0.98] | 0.95x [0.91, 0.98] | slower |
| music/native(240s)/30s/w1000 | 46.270 us | 42.270 us | 1.09x [1.05, 1.12] | 1.10x [1.05, 1.12] | faster |
| music/native(240s)/1s/w1000 | 36.105 us | 38.229 us | 0.94x [0.91, 0.97] | 0.92x [0.90, 0.96] | slower |
| music/2h-tiled(7200s)/whole/w1000 | 2.612 ms | 61.312 us | 42.59x [39.76, 44.48] | 42.47x [39.28, 43.90] | faster |
| music/2h-tiled(7200s)/whole/w3840 | 2.707 ms | 221.917 us | 12.20x [11.65, 12.48] | 12.14x [11.55, 12.47] | faster |
| music/2h-tiled(7200s)/1/4/w1000 | 679.605 us | 58.542 us | 11.61x [10.83, 11.99] | 11.62x [10.71, 11.93] | faster |
| music/2h-tiled(7200s)/1/16/w1000 | 191.917 us | 58.270 us | 3.29x [3.24, 3.49] | 3.25x [3.25, 3.48] | faster |
| music/2h-tiled(7200s)/30s/w1000 | 48.980 us | 44.834 us | 1.09x [1.08, 1.10] | 1.09x [1.09, 1.11] | faster |
| music/2h-tiled(7200s)/1s/w1000 | 35.938 us | 38.770 us | 0.93x [0.89, 0.96] | 0.92x [0.90, 0.96] | slower |
| **geomean** | | | **2.49x [2.46, 2.51]** | 2.48x [2.45, 2.51] | |

### silence  (publishable: true, contended: false, load Some(3.68)->Some(3.68), 40 runs/side)

| input | old median | new median | wall speedup [95% CI] | cpu speedup [95% CI] | verdict |
|---|---|---|---|---|---|
| silence/native(120s)/whole/w1000 | 18.916 us | 7.729 us | 2.45x [2.41, 2.47] | 2.50x [2.38, 2.50] | faster |
| silence/native(120s)/whole/w3840 | 23.166 us | 19.083 us | 1.21x [1.20, 1.22] | 1.20x [1.15, 1.26] | faster |
| silence/native(120s)/1/4/w1000 | 6.250 us | 5.291 us | 1.18x [1.17, 1.20] | 1.17x [1.17, 1.40] | faster |
| silence/native(120s)/1/16/w1000 | 3.750 us | 4.021 us | 0.93x [0.91, 0.96] | 1.00x [0.80, 1.00] | slower |
| silence/native(120s)/30s/w1000 | 6.250 us | 5.291 us | 1.18x [1.17, 1.19] | 1.40x [1.00, 1.40] | faster |
| silence/native(120s)/1s/w1000 | 3.459 us | 3.959 us | 0.87x [0.86, 0.88] | 1.00x [1.00, 1.00] | slower |
| silence/2h-tiled(7200s)/whole/w1000 | 2.389 ms | 15.354 us | 155.58x [142.31, 162.30] | 149.16x [140.35, 159.23] | faster |
| silence/2h-tiled(7200s)/whole/w3840 | 2.177 ms | 51.416 us | 42.34x [36.81, 43.27] | 41.88x [37.84, 42.81] | faster |
| silence/2h-tiled(7200s)/1/4/w1000 | 539.104 us | 12.938 us | 41.67x [37.34, 42.84] | 41.54x [36.00, 41.69] | faster |
| silence/2h-tiled(7200s)/1/16/w1000 | 97.541 us | 10.291 us | 9.48x [9.11, 9.65] | 8.91x [8.82, 9.80] | faster |
| silence/2h-tiled(7200s)/30s/w1000 | 6.042 us | 4.959 us | 1.22x [1.20, 1.23] | 1.20x [1.00, 1.40] | faster |
| silence/2h-tiled(7200s)/1s/w1000 | 3.458 us | 3.958 us | 0.87x [0.87, 0.88] | 1.00x [0.88, 1.00] | slower |
| **geomean** | | | **3.81x [3.74, 3.83]** | 3.93x [3.76, 4.01] | |

### soundcraft  (publishable: true, contended: false, load Some(3.68)->Some(3.68), 40 runs/side)

| input | old median | new median | wall speedup [95% CI] | cpu speedup [95% CI] | verdict |
|---|---|---|---|---|---|
| sc_tone_mp3/native(300s)/whole/w1000 | 126.000 us | 39.500 us | 3.19x [3.02, 3.24] | 3.15x [3.00, 3.26] | faster |
| sc_tone_mp3/native(300s)/whole/w3840 | 196.479 us | 134.729 us | 1.46x [1.44, 1.47] | 1.46x [1.45, 1.47] | faster |
| sc_tone_mp3/native(300s)/1/4/w1000 | 49.500 us | 35.688 us | 1.39x [1.38, 1.39] | 1.39x [1.36, 1.39] | faster |
| sc_tone_mp3/native(300s)/1/16/w1000 | 28.896 us | 29.084 us | 0.99x [0.95, 1.07] | 1.00x [0.97, 1.07] | inconclusive (CI includes 1.0) |
| sc_tone_mp3/native(300s)/30s/w1000 | 32.270 us | 30.083 us | 1.07x [1.05, 1.08] | 1.07x [1.05, 1.10] | faster |
| sc_tone_mp3/native(300s)/1s/w1000 | 21.688 us | 27.646 us | 0.78x [0.77, 0.79] | 0.79x [0.79, 0.79] | slower |
| sc_tone_mp3/2h-tiled(7200s)/whole/w1000 | 2.481 ms | 59.167 us | 41.93x [35.97, 47.69] | 42.41x [35.73, 47.75] | faster |
| sc_tone_mp3/2h-tiled(7200s)/whole/w3840 | 2.548 ms | 200.230 us | 12.72x [11.87, 13.74] | 12.67x [11.93, 13.67] | faster |
| sc_tone_mp3/2h-tiled(7200s)/1/4/w1000 | 637.250 us | 47.541 us | 13.40x [10.65, 13.66] | 13.28x [10.56, 13.59] | faster |
| sc_tone_mp3/2h-tiled(7200s)/1/16/w1000 | 176.979 us | 42.334 us | 4.18x [3.84, 4.29] | 4.14x [3.83, 4.34] | faster |
| sc_tone_mp3/2h-tiled(7200s)/30s/w1000 | 32.584 us | 30.084 us | 1.08x [1.07, 1.17] | 1.10x [1.06, 1.17] | faster |
| sc_tone_mp3/2h-tiled(7200s)/1s/w1000 | 21.604 us | 27.646 us | 0.78x [0.77, 0.84] | 0.79x [0.77, 0.84] | slower |
| sc_tone_ogg/native(300s)/whole/w1000 | 126.480 us | 39.166 us | 3.23x [3.02, 3.26] | 3.22x [3.02, 3.26] | faster |
| sc_tone_ogg/native(300s)/whole/w3840 | 197.458 us | 136.000 us | 1.45x [1.45, 1.46] | 1.46x [1.45, 1.46] | faster |
| sc_tone_ogg/native(300s)/1/4/w1000 | 49.938 us | 35.792 us | 1.40x [1.39, 1.40] | 1.39x [1.39, 1.39] | faster |
| sc_tone_ogg/native(300s)/1/16/w1000 | 32.833 us | 31.041 us | 1.06x [1.05, 1.06] | 1.06x [1.03, 1.06] | faster |
| sc_tone_ogg/native(300s)/30s/w1000 | 35.709 us | 30.895 us | 1.16x [1.15, 1.16] | 1.16x [1.16, 1.16] | faster |
| sc_tone_ogg/native(300s)/1s/w1000 | 25.521 us | 26.959 us | 0.95x [0.94, 0.95] | 0.96x [0.94, 0.96] | slower |
| sc_tone_ogg/2h-tiled(7200s)/whole/w1000 | 2.475 ms | 53.188 us | 46.52x [41.57, 50.63] | 46.64x [41.31, 49.92] | faster |
| sc_tone_ogg/2h-tiled(7200s)/whole/w3840 | 2.546 ms | 193.145 us | 13.18x [12.78, 13.84] | 13.19x [12.76, 13.80] | faster |
| sc_tone_ogg/2h-tiled(7200s)/1/4/w1000 | 640.042 us | 48.834 us | 13.11x [12.16, 13.83] | 13.05x [12.16, 13.65] | faster |
| sc_tone_ogg/2h-tiled(7200s)/1/16/w1000 | 176.958 us | 41.480 us | 4.27x [3.74, 4.33] | 4.21x [3.69, 4.32] | faster |
| sc_tone_ogg/2h-tiled(7200s)/30s/w1000 | 35.791 us | 30.833 us | 1.16x [1.16, 1.17] | 1.16x [1.16, 1.16] | faster |
| sc_tone_ogg/2h-tiled(7200s)/1s/w1000 | 25.625 us | 27.042 us | 0.95x [0.94, 0.95] | 0.96x [0.96, 0.96] | slower |
| sc_tone_m4a/native(300s)/whole/w1000 | 126.438 us | 39.292 us | 3.22x [3.10, 3.24] | 3.17x [3.10, 3.26] | faster |
| sc_tone_m4a/native(300s)/whole/w3840 | 197.479 us | 135.917 us | 1.45x [1.44, 1.46] | 1.46x [1.45, 1.47] | faster |
| sc_tone_m4a/native(300s)/1/4/w1000 | 50.041 us | 35.500 us | 1.41x [1.40, 1.41] | 1.39x [1.39, 1.39] | faster |
| sc_tone_m4a/native(300s)/1/16/w1000 | 34.167 us | 32.270 us | 1.06x [1.05, 1.06] | 1.08x [1.03, 1.09] | faster |
| sc_tone_m4a/native(300s)/30s/w1000 | 35.792 us | 30.875 us | 1.16x [1.15, 1.16] | 1.16x [1.16, 1.16] | faster |
| sc_tone_m4a/native(300s)/1s/w1000 | 25.396 us | 28.834 us | 0.88x [0.87, 0.89] | 0.90x [0.86, 0.91] | slower |
| sc_tone_m4a/2h-tiled(7200s)/whole/w1000 | 2.484 ms | 58.334 us | 42.59x [38.27, 47.87] | 42.11x [38.22, 47.80] | faster |
| sc_tone_m4a/2h-tiled(7200s)/whole/w3840 | 2.543 ms | 199.708 us | 12.74x [11.46, 13.84] | 12.98x [11.46, 13.83] | faster |
| sc_tone_m4a/2h-tiled(7200s)/1/4/w1000 | 635.729 us | 49.021 us | 12.97x [11.57, 13.83] | 12.72x [11.78, 13.83] | faster |
| sc_tone_m4a/2h-tiled(7200s)/1/16/w1000 | 177.021 us | 42.625 us | 4.15x [3.91, 4.32] | 4.21x [3.89, 4.32] | faster |
| sc_tone_m4a/2h-tiled(7200s)/30s/w1000 | 35.854 us | 31.125 us | 1.15x [1.15, 1.16] | 1.14x [1.12, 1.16] | faster |
| sc_tone_m4a/2h-tiled(7200s)/1s/w1000 | 25.459 us | 28.875 us | 0.88x [0.87, 0.89] | 0.90x [0.86, 0.90] | slower |
| **geomean** | | | **2.77x [2.74, 2.79]** | 2.77x [2.74, 2.79] | |

### claim-shape-shipped-scan  (publishable: true, contended: false, load Some(3.68)->Some(3.54), 40 runs/side)

| input | old median | new median | wall speedup [95% CI] | cpu speedup [95% CI] | verdict |
|---|---|---|---|---|---|
| synthetic-2h/7200s/w1000 | 2.490 ms | 61.022 us | 40.81x [40.08, 41.45] | 40.83x [40.08, 41.53] | faster |
| synthetic-2h/1800s/w1000 | 652.896 us | 60.062 us | 10.87x [10.51, 10.99] | 10.88x [10.45, 10.90] | faster |
| synthetic-2h/300s/w1000 | 134.458 us | 49.208 us | 2.73x [2.67, 2.74] | 2.70x [2.65, 2.76] | faster |
| synthetic-2h/30s/w1000 | 45.938 us | 42.334 us | 1.09x [1.07, 1.09] | 1.07x [1.07, 1.10] | faster |
| synthetic-2h/1s/w1000 | 33.271 us | 39.875 us | 0.83x [0.83, 0.85] | 0.85x [0.82, 0.85] | slower |
| **geomean** | | | **4.06x [4.01, 4.08]** | 4.05x [4.00, 4.08] | |

### claim-shape-minmax-fold  (publishable: true, contended: false, load Some(3.54)->Some(3.54), 40 runs/side)

| input | old median | new median | wall speedup [95% CI] | cpu speedup [95% CI] | verdict |
|---|---|---|---|---|---|
| synthetic-2h/7200s/w1000 | 189.728 us | 59.458 us | 3.19x [3.14, 3.24] | 3.17x [3.12, 3.24] | faster |
| synthetic-2h/1800s/w1000 | 80.771 us | 59.105 us | 1.37x [1.35, 1.38] | 1.37x [1.34, 1.38] | faster |
| synthetic-2h/300s/w1000 | 58.395 us | 48.834 us | 1.20x [1.19, 1.20] | 1.20x [1.18, 1.20] | faster |
| synthetic-2h/30s/w1000 | 43.354 us | 42.416 us | 1.02x [1.01, 1.03] | 1.02x [1.00, 1.05] | faster |
| synthetic-2h/1s/w1000 | 35.687 us | 39.770 us | 0.90x [0.89, 0.91] | 0.90x [0.90, 0.90] | slower |
| **geomean** | | | **1.37x [1.36, 1.37]** | 1.37x [1.36, 1.38] | |

### claim-shape-shipped-scan-columns-only  (publishable: true, contended: false, load Some(3.54)->Some(3.54), 40 runs/side)

| input | old median | new median | wall speedup [95% CI] | cpu speedup [95% CI] | verdict |
|---|---|---|---|---|---|
| synthetic-2h/7200s/w1000/columns-only | 2.520 ms | 22.750 us | 110.78x [64.97, 163.21] | 105.64x [64.13, 157.43] | faster |
| synthetic-2h/1800s/w1000/columns-only | 560.813 us | 15.542 us | 36.08x [22.72, 42.23] | 36.13x [22.73, 40.86] | faster |
| synthetic-2h/300s/w1000/columns-only | 54.125 us | 10.209 us | 5.30x [2.95, 6.08] | 5.19x [2.81, 5.90] | faster |
| synthetic-2h/30s/w1000/columns-only | 5.667 us | 4.479 us | 1.27x [1.14, 1.33] | 1.20x [1.20, 1.20] | faster |
| synthetic-2h/1s/w1000/columns-only | 3.021 us | 3.562 us | 0.85x [0.60, 1.19] | 0.75x [0.60, 1.12] | inconclusive (CI includes 1.0) |
| **geomean** | | | **7.44x [6.06, 8.27]** | 7.08x [5.88, 7.93] | |

### claim-shape-minmax-fold-columns-only  (publishable: true, contended: false, load Some(3.54)->Some(3.54), 40 runs/side)

| input | old median | new median | wall speedup [95% CI] | cpu speedup [95% CI] | verdict |
|---|---|---|---|---|---|
| synthetic-2h/7200s/w1000/columns-only | 172.645 us | 13.729 us | 12.57x [12.35, 12.73] | 12.32x [12.14, 12.43] | faster |
| synthetic-2h/1800s/w1000/columns-only | 52.708 us | 12.042 us | 4.38x [4.34, 4.44] | 4.42x [4.08, 4.42] | faster |
| synthetic-2h/300s/w1000/columns-only | 13.458 us | 8.791 us | 1.53x [1.49, 1.55] | 1.56x [1.44, 1.56] | faster |
| synthetic-2h/30s/w1000/columns-only | 4.500 us | 4.500 us | 1.00x [1.00, 1.01] | 0.80x [0.80, 1.00] | inconclusive (CI includes 1.0) |
| synthetic-2h/1s/w1000/columns-only | 3.417 us | 3.500 us | 0.98x [0.98, 0.99] | 1.14x [0.75, 1.33] | slower |
| **geomean** | | | **2.42x [2.40, 2.43]** | 2.39x [2.24, 2.58] | |
