# Basic Finner 実測データの出典

Army-Navy Basic Finner（10 口径の円すい・円柱に矩形フィン 4 枚）の実測データを、IgnisYeet の空力手法（部品積み上げ法、パネル法、CFD の Euler と RANS、6 自由度の減衰とロールの項）の妥当性確認に使う。
この節は、使ったデータの出典、読み取り方、信頼性をまとめる。

* `measured.csv`：実測値（1 行 1 量の縦長の形式）
* `geometry.json`：形状、基準量、質量特性、試験条件
* `first_look_barrowman_panel.csv`：部品積み上げ法とパネル法の値と実測値の比較（下の「最初の比較」）

ページ番号は、断りのない限り PDF のページ（スキャンの通し番号）である。

## 使った出典

### [D97] Dupuis & Hathaway, DREV-TM-9703（1997）— 主たる出典

* A. D. Dupuis and W. Hathaway, "Aeroballistic Range Tests of the Basic Finner Reference Projectile at Supersonic Velocities", DREV-TM-9703, Defence Research Establishment Valcartier, August 1997. Unlimited distribution. DTIC ADA636861.
* 入手先：DTIC の PDF（https://apps.dtic.mil/sti/tr/pdf/ADA636861.pdf ）は自動取得を拒否するので、Internet Archive の写し（https://web.archive.org/web/2020/https://apps.dtic.mil/dtic/tr/fulltext/u2/a636861.pdf 、153 ページ。https://archive.org/details/DTIC_ADA636861 にも同じ報告書がある）を使った。
* 内容：DREV の弾道射場での自由飛行試験 26 発（データは 25 発）。Mach 1.05 から 4.47、長さ基準の Reynolds 数 7.0e6 から 3.0e7（Table V）。フィンのカント 0、2、4 deg（Table III）。線形理論（Table VI、VII）と 6 自由度の最尤法（単発 Table VIII、複数発の同時当てはめ Table IX）で係数を求めている。
* 使った表：
  * Table IX（p. 48、49）：6 自由度の複数発同時当てはめ。8 つの Mach 数（1.077、1.293、1.832、2.375、2.718、3.147、3.734、4.300）について CX0、CX2、CNα、Cmα、Cmα3、CmαM、Cmq、Cnpα、Clp、CXM。表の値をそのまま写し、PDF の画像と突き合わせて確かめた。括弧内の確率誤差（%）は `probable_error_pct`、`(*)`（その当てはめで固定した値）は `held_fixed = 1` にした。
  * Table VII（p. 43）：線形理論の単発の値（CD0、CDSQ、CNα、Cmα、Cmq、Cnpα）。ロールの当てはめはカントのモーメントを含まないので、この表の Clp は報告書自身が無意味としている（Sec. 5.1）。そのため Clp は写していない。
  * Table I（p. 36）、Table II（p. 37）、Table V（p. 41）、Fig. 2（p. 53）、Sec. 3.1（p. 16）：形状、質量特性、射場の条件（`geometry.json`）。
* 基準：直径 d = 30 mm、基準面積 πd²/4。ピッチと Magnus のモーメントの基準点は「長さの 55 %（5.50 口径）」（Sec. 5.2、p. 22）。CNα と Cmα は 1/rad。
* 減衰の無次元化：報告書は Cmq と Clp の角速度の無次元化（qd/V か qd/(2V) か）を書いていない。`measured.csv` では qd/(2V)、pd/(2V) とした。根拠は次の 2 つである。
  1. Franzmann ら（[F22]）は、この報告書の続編 [D02] の自由飛行の Cmq + Cmα̇ を、qD/(2V∞) で無次元化した自分の風洞の値と同じ軸に（重心の換算だけをして）描き、両者の大きさが一致している（−300 から −440）。
  2. Regan の風洞の Clp（[R64]、下の換算）が、pd/(2V) の Clp として Table IX の値と 10 % 程度で一致する。pd/V なら 2 倍ずれる。
  この点は推定であり、`normalization` の列に `[inferred]` と書いた。
* Cmq の列は自由飛行の同定なので Cmq + Cmα̇ の和である（射場のデータでは両者を分けられない）。
* 注意点：
  * Mach 1.832、2.375、2.718、3.147 の群では、CX0 を発ごとに解いた（Sec. 5.2、p. 23）。表の群の値には `(*)` が付いている。Table VII の CD0 の群ごとの平均と一致する（例：Mach 1.8 の 3 発 0.593、0.596、0.565 の平均 0.585）。
  * 群の Mach 数は群の発の射程中央の Mach 数の平均である。群の中での Mach 数の変化は CmαM と CXM で表されている。
  * DA13、DA22、DA23（DA95030613、DA95030622、DA95031523）は線形理論で歳差運動が減衰しない（Sec. 5.1、7.0。カントによる回転と Magnus の不安定）。`measured.csv` の Table VII の行に注記した。
  * 6 自由度の Cmq は Mach 2.4 の群が傾向から外れる（Sec. 5.3）。Cmq の確率誤差は 1 から 6 %、発ごとのばらつきは 20 % 程度ある。
  * 遷音速のピーク（Mach 1.25 付近）は試験で捉えていない（Sec. 5.3）。
  * DA14（約 1520 m/s）はフィンが融けてデータがない。DA01 と DA11 にもわずかな融損の疑いがある（Sec. 5.2）。Mach 4.3 の群は DA95031523 と DA95030111 で、DA01（DA95030101）は含まない。
* 信頼性：Table IX の主要な係数（CX0、CNα、Cmα、Clp）は確率誤差 0 から 4 %。著者らは BRL の射場の値（MacAllister 1955、Nicolaides & Bolz 1952）と比べて「よく一致する」としている（Sec. 6.0、Fig. 17）。独立な風洞の値（[SG60] Fig. 29。目で読んで Mach 2.0–2.5 で CNα ≈ 10、Mach 1.6 で約 13）とも合う。

### [R64] Regan, NAVORD Report 6652（1964）— ロール減衰

* F. J. Regan, "Roll Damping Moment Measurements for the Basic Finner at Subsonic and Supersonic Speeds", NAVORD Report 6652 (Aerodynamics Research Report 59), U.S. Naval Ordnance Laboratory, White Oak, 20 March 1964. DTIC AD0600975（https://archive.org/details/DTIC_AD0600975 ）。
* 内容：NOL の風洞での回転減衰（spin decay）試験。迎角 0 の Clp を Mach 0.22、0.77 と 1.53 から 4.81 の 8 点で、迎角の影響を Mach 0.22、0.77、2.54、4.10 で測っている。
* 使った表：Table 2（p. 17）、迎角 0 の超音速の Clp（最大、最小、平均。50–150 rps で 10 rps ごと）。PDF の画像で値を確かめた（OCR では 1.53 の最大値と 4.81 の最大値が読み違えられていた）。
* 定義（p. 6、SYMBOLS）：Cl = L / (q Ap b n)、Clp = ∂Cl/∂(pb/(2V))。Ap はフィン 1 枚の面積、b はフィンのスパン、n はフィンの数。表の値は大きさ（正）で書かれている。
* 換算：b を全幅 3 口径（「overall span of three calibers」、p. 8）、Ap = 1 口径²、n = 4 とすると、d と πd²/4 を基準にした pd/(2V) あたりの Clp は Regan の値の n Ap b² / (S d²) = 144/π ≈ 45.84 倍（符号は負）になる。b を 1 枚のスパン（1 口径）とすると 5.09 倍になり、[D97] の値と 4 倍以上ずれる。全幅とした換算は [D97] と 10 % 程度で合う（Mach 2.03 で 19.3 に対し Mach 1.832 の 22.35、Mach 2.375 の 18.31）ので、全幅と判断した。`measured.csv` には原表の値（`Clp_regan`）と換算した値（`Clp`）の両方を置いた。最大値と最小値の幅の半分を平均値に対する % で `probable_error_pct` に入れた。
* 迎角 0 の亜音速（Mach 0.22、0.77）の Clp と迎角の影響は図（Fig. 3–7）にしかない。今回は読み取っていない。
* 信頼性：風洞の値は自由飛行の値よりわずかに小さく、迎角とともに大きくなると著者は述べている（p. 13）。軸受けの摩擦は、測定部を真空にした回転減衰を風袋として差し引いている（p. 11、12）。

## 図だけにあり、今回は数値にしていない出典

* [SG60] I. Shantz and R. T. Groves, "Dynamic and Static Stability Measurements of the Basic Finner at Supersonic Speeds", NAVORD Report 4516, U.S. Naval Ordnance Laboratory, 14 January 1960. DTIC AD0245560（https://archive.org/details/DTIC_AD0245560 ）。NOL の超音速風洞 No. 1。静的な CNα、Cmα、xcp（Mach 1.58–3.86）、Cmq + Cmα̇（Mach 1.58–3.24、重心 5 口径と 6 口径）。値は Fig. 16–30 の図だけで表はない。図には BRL の射場の値も重ねてある。必要になれば Fig. 26（Cmα）、Fig. 29（CNα）、Fig. 30（xcp）を読み取れる（読み取りの誤差は CNα で ±0.5 程度の見込み）。
* [F22] C. Franzmann, F. Leopold and C. Mundt, "Low-Interference Wind Tunnel Measurement Technique for Pitch Damping Coefficients at Transonic and Low Supersonic Mach Numbers", Aerospace 9(2):51, 2022, doi:10.3390/aerospace9020051（オープンアクセス）。ISL の風洞（ワイヤ懸垂、D = 20 mm）で Cmq + Cmα̇ を Mach 0.6–2.0（亜音速を含む）で測っている。値は Fig. 10、11 の図だけ（表は試験条件の Table 1 だけ）。qD/(2V∞) で無次元化している。
* [B24] I. Bunescu, M.-G. Stoican and M.-V. Hothazie, "Experimental Determination of Pitch Damping Coefficient Using Free Oscillation Method", Aerospace 11(7):579, 2024, doi:10.3390/aerospace11070579（オープンアクセス）。INCAS の風洞（D = 60 mm）、Mach 0.4–3.5、回転中心 5.5 口径と 6.1 口径。結果は図だけ。
* [BS14] A. Bhagwandin and J. Sahu, "Numerical Prediction of Pitch Damping Stability Derivatives for Finned Projectiles", Journal of Spacecraft and Rockets 51(5), 2014, doi:10.2514/1.A32734。CFD（RANS）と [D02] の自由飛行・風洞の値の比較の図。入手していない（有料）。
* S. M. Murman, AIAA Paper 2007-0074：CFD の動的微係数の論文（依頼で名前の挙がったもの）。入手・確認していない。
* Kiceniuk（Caltech Hydrodynamics Laboratory Report E-73.3, 1957、DTIC AD0142815）と Caltech の水洞の静的な力（1957、DTIC AD0146516）：水中（非圧縮）の試験。スキャンの質が悪く OCR では読めない。図だけ。

## 入手できなかった出典

* [D02] A. D. Dupuis, "Aeroballistic Range and Wind Tunnel Tests of the Basic Finner Reference Projectile from Subsonic to High Supersonic Velocities", DRDC Valcartier TM 2002-136, October 2002。亜音速から Mach 4.5 までの射場と風洞の値を含む、この模型の最も網羅的な報告書である。DTIC、DRDC の公開リポジトリ、Internet Archive のいずれでも本文を見つけられなかった。**亜音速の静的な係数（CNα、Cmα、CX0）の表はこの報告書にしかなく、今回のデータには亜音速の静的な値がない。** 入手するには、DRDC の Canadian Defence Information Database（https://cradpdf.drdc-rddc.gc.ca/ ）で報告書番号を検索するか、DRDC Valcartier の図書室に請求する。
* L. C. MacAllister, "The Aerodynamic Properties of a Simple Non-Rolling Finned Cone-Cylinder Configuration Between Mach Numbers 1.0 and 2.5", BRL Report No. 934, 1955。BRL の射場の元の値。見つけられなかった（[D97] Fig. 17 と [SG60] の図に重ねてある）。
* H. Murthy, "Subsonic and Transonic Roll Damping Measurement on Basic Finner", Journal of Spacecraft and Rockets 19(1):86–87, 1982。亜音速の Clp。有料で入手していない。

## 最初の比較（部品積み上げ法とパネル法、Mach 1.08–4.30）

`first_look_barrowman_panel.csv` は、`examples/basic_finner/basic_finner.toml` で作った係数表（迎角 0）を Table IX の Mach 数で線形補間し、実測値と並べたものである。
「extracted」は STL から抽出したフィン、「exactfins」は `rocket.fin_override` で寸法を図面どおり（根元と先端の翼弦 30 mm、スパン 30 mm、後退 0、厚さ 2.4 mm、前縁 270 mm）にしたものである。
Cmα は 0.165 m まわり、xcp は口径、Cmq は減衰の和の重心まわりの値 −2K/d²、「Cmq+lag」はそれに遅れの和の Cmα̇ の 2 倍（見かけの質量の遅れと対流の回転の項。飛翔計算が加える）を足した値である。

Mach 1.83–4.30 での傾向（exactfins）は次のとおりである（「小さい」「大きい」は大きさの比較）。

* CNα：部品積み上げ法が実測より 20–35 % 小さく、パネル法は 33–42 % 小さい。Mach 1.08、1.29 では部品積み上げ法も 31 % 小さい。
* xcp：部品積み上げ法は ±2.5 % 以内、パネル法は 2–6 % 前寄り。Cmα は CNα の不足をそのまま受けて 21–37 %（部品積み上げ法）小さい。
* CX0：部品積み上げ法が 19–30 % 小さく、パネル法が 10–19 % 小さい。
* Clp：Mach 1.8–3.2 で −8 から +6 %、Mach 3.7、4.3 で 13–15 % 小さい。Mach 1.08、1.29 では 45–91 % 大きい。
* Cmq + Cmα̇：減衰の和だけでは 24–39 % 小さく、遅れの項を足すと部品積み上げ法で −14 から +8 %、パネル法で −0.3 から +18 %（`diff_pct` は (モデル − 実測)/|実測|。負の係数では正が「大きさが小さい」）。

## 文献の BibTeX（doc/source/references.bib への追加案）

```bibtex
@techreport{dupuis1997basicfinner,
  author      = {Dupuis, Alain D. and Hathaway, Wayne},
  title       = {Aeroballistic Range Tests of the Basic Finner Reference Projectile at Supersonic Velocities},
  institution = {Defence Research Establishment Valcartier},
  address     = {Valcartier, Qu{\'e}bec, Canada},
  type        = {Technical Memorandum},
  number      = {DREV-TM-9703},
  year        = {1997},
  month       = aug,
  note        = {DTIC ADA636861}
}

@techreport{regan1964rolldamping,
  author      = {Regan, Frank J.},
  title       = {Roll Damping Moment Measurements for the Basic Finner at Subsonic and Supersonic Speeds},
  institution = {U.S. Naval Ordnance Laboratory},
  address     = {White Oak, Maryland},
  type        = {NAVORD Report},
  number      = {6652},
  year        = {1964},
  month       = mar,
  note        = {Aerodynamics Research Report 59, DTIC AD0600975}
}

@techreport{shantz1960basicfinner,
  author      = {Shantz, Irving and Groves, Robert T.},
  title       = {Dynamic and Static Stability Measurements of the Basic Finner at Supersonic Speeds},
  institution = {U.S. Naval Ordnance Laboratory},
  address     = {White Oak, Maryland},
  type        = {NAVORD Report},
  number      = {4516},
  year        = {1960},
  month       = jan,
  note        = {DTIC AD0245560}
}

@techreport{dupuis2002basicfinner,
  author      = {Dupuis, Alain D.},
  title       = {Aeroballistic Range and Wind Tunnel Tests of the Basic Finner Reference Projectile from Subsonic to High Supersonic Velocities},
  institution = {Defence R\&D Canada -- Valcartier},
  address     = {Valcartier, Qu{\'e}bec, Canada},
  type        = {Technical Memorandum},
  number      = {DRDC Valcartier TM 2002-136},
  year        = {2002},
  month       = oct
}

@article{franzmann2022pitchdamping,
  author  = {Franzmann, Christian and Leopold, Friedrich and Mundt, Christian},
  title   = {Low-Interference Wind Tunnel Measurement Technique for Pitch Damping Coefficients at Transonic and Low Supersonic Mach Numbers},
  journal = {Aerospace},
  volume  = {9},
  number  = {2},
  pages   = {51},
  year    = {2022},
  doi     = {10.3390/aerospace9020051}
}

@article{bunescu2024pitchdamping,
  author  = {Bunescu, Ionu{\c{t}} and Stoican, Mih{\u{a}}i{\c{t}}{\u{a}}-Gilbert and Hothazie, Mihai-Vl{\u{a}}du{\c{t}}},
  title   = {Experimental Determination of Pitch Damping Coefficient Using Free Oscillation Method},
  journal = {Aerospace},
  volume  = {11},
  number  = {7},
  pages   = {579},
  year    = {2024},
  doi     = {10.3390/aerospace11070579}
}

@article{bhagwandin2014pitchdamping,
  author  = {Bhagwandin, V. A. and Sahu, J.},
  title   = {Numerical Prediction of Pitch Damping Stability Derivatives for Finned Projectiles},
  journal = {Journal of Spacecraft and Rockets},
  volume  = {51},
  number  = {5},
  year    = {2014},
  doi     = {10.2514/1.A32734}
}
```

`bhagwandin2014pitchdamping` は本文を入手していない。巻・号（51(5)）は追加の前に DOI で確かめる。
