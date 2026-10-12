.. _page-testing:

テスト
======

この章は、IgnisYeet のテストが何を、どのように確かめているかを述べる。
テストは、式の実装が正しいか、設定やファイルの扱いが仕様どおりか、ドキュメントとコードが食い違っていないかを、変更のたびに自動で確かめる仕組みである。
実測との比較や、手法どうしの比較の結果は :ref:`page-vv` と :ref:`page-vv-validation` にまとめてある。この章はそれらと違い、テストのコードそのものを対象とする。
関数名とファイル名は、リポジトリで該当のテストを探せるように挙げる。

数え方は次のとおりである。
テスト関数は、Rust の ``#[test]`` を付けた関数の数である。
``cargo test -- --list`` の一覧には、一時ディレクトリの後始末を確かめる共通の 2 件（``testutil/tmp.rs``）が 8 つのテストバイナリに重複して現れる。
この章の数は、その重複を除いて数える。

.. _sec-testing-overview:

テストの種類と実行のしかた
--------------------------

テストは、確かめる対象と実行の条件で五つの層に分かれる。
:numref:`fig-test-layers` は、その対応である。
青の層は ``cargo test --release`` や CI で毎回実行される。橙の層は、外部のツールが必要なので既定では実行されない。

.. _fig-test-layers:

.. figure:: /_generated/tikz/test_layers.*
   :width: 100%

   テストの層。青は CI で毎回実行し、橙は既定では実行しない。

.. csv-table:: テストの実行コマンド
   :delim: ¦
   :header-rows: 1
   :widths: 46 54

   コマンド¦内容
   ``cargo test --release``¦Rust の単体テストと結合テスト。``#[ignore]`` のテストは含まない
   ``cargo test --release -p cfd -- --ignored``¦SU2、gmsh、MPI を使う CFD のテスト。ツールの場所は環境変数 ``IGNISYEET_CFD_PREFIX`` で渡す
   ``cargo test --release -- --ignored``¦``#[ignore]`` を付けたテストをすべて実行する（計時と集計の 2 件を含む）
   ``uv run --project python python -m unittest discover -s python``¦作図スクリプト ``python/plot.py`` のテスト
   ``bash scripts/test-install-codes.sh``¦インストーラ ``install.sh`` のエラーコードの検査

CI（``.github/workflows/ci.yml``）は、``cargo test --release --locked``、作図のテスト、インストーラのコードの検査を実行する。
続けて例題を ``ignisyeet geom``、``aero``、``sim``、``dispersion`` で通しで計算し、``sim.max_time`` を短くした飛翔の終了ステータスが 3 になることを確かめる。
リリースのワークフロー（``release.yml``）も、Linux（x86_64、musl）と macOS（Intel と Apple Silicon）のビルドで ``cargo test --release --locked`` を実行する。
どちらも ``#[ignore]`` のテストを実行しないので、CFD のソルバを実際に走らせるテストは CI の対象外である。

.. _tbl-test-counts:

.. csv-table:: クレートごとのテスト関数の数
   :delim: ¦
   :header-rows: 1
   :widths: 46 18 18 18

   クレート¦既定で実行¦``#[ignore]``¦計
   ``aero``¦58¦0¦58
   ``geom``¦24¦0¦24
   ``panel``¦35¦1¦36
   ``sim``¦137¦0¦137
   ``cfd``¦165¦16¦181
   ``cli``¦156¦1¦157
   Rust の合計¦575¦18¦593
   Python（``python/test_plot.py``）¦41¦0¦41
   shell（``scripts/test-install-codes.sh``）¦1 本¦¦

各クレートの内容は :ref:`sec-testing-crates` に、Python と shell は :ref:`sec-testing-python` に述べる。

``cli`` の数には、``crates/cli/tests`` の 3 件（設定キーとエラーコードのドキュメントとの対応）を含む。
``cfd`` の 16 件は ``crates/cfd/tests`` にあり、すべて ``#[ignore]`` である。
:numref:`fig-test-counts` は、同じ数を棒グラフにしたものである。数は ``cargo test --release --workspace -- --list`` と ``-- --list --ignored`` の出力による（出所は ``doc/data/testing/test_counts.csv`` の先頭の行にある）。

.. _fig-test-counts:

.. figure:: /_generated/plots/test_counts.*
   :width: 80%

   クレートごとのテスト関数の数。橙は ``#[ignore]`` のテストである。

既定で実行されるテストの所要時間は、1 回の測定で、``cfd`` が約 95 秒、``cli`` が約 17 秒、``panel`` が約 13 秒、そのほかのクレートが 2 秒以内だった。
``cfd`` のテストには、ソルバを擬似のスクリプトに置き換えた実行制御のテスト（:ref:`sec-testing-double`）が多い。

.. _sec-testing-patterns:

テストの型
----------

テストは、何を基準に正否を決めるかによって、いくつかの型に分けられる。
この節は、型ごとにリポジトリの実際のテストを例に挙げる。
判定の許容幅は、後ろの :ref:`sec-testing-margins` でまとめて見る。

.. _sec-testing-analytic:

解析解と理論式との一致
~~~~~~~~~~~~~~~~~~~~~~

閉じた式で答えが分かる問題を解かせ、その答えと比べる型である。モデルの式が正しく実装されているかの検証にあたる。
標準大気の値は、文献値（US Standard Atmosphere 1976）に当たる数を直接書いて比べる（``crates/aero/src/atmosphere.rs``\ 、``standard_values``）。

.. code-block:: rust

   let s = us76(0.0);
   assert!((s.density - 1.225).abs() < 1e-3);
   assert!((s.sound_speed - 340.29).abs() < 0.1);
   let t = us76(11_000.0);
   assert!((t.temperature - 216.77).abs() < 0.2);

超音速の局所傾斜法で使う衝撃波の関係式は、教科書の値と比べる（``crates/panel/src/supersonic.rs``\ 、``oblique_shock_and_pm``）。
Mach 2、くさび角 10° の斜め衝撃波の角度が 39.3°、圧力係数が 0.2525、Prandtl–Meyer 角が 26.38° であること、くさび角 30° では衝撃波が付着しない（解がない）こと、微小な膨張角では線形理論 :math:`-2\delta/\sqrt{M^2-1}` に 2 % 以内で一致することを確かめる。
同じファイルの ``cone_10deg_mach2`` は、Mach 2 の円すい（半頂角 10°）の圧力係数を、Taylor–Maccoll の方程式を衝撃波の角度から直接解いた値（0.098）と比べる。

部品積み上げ法は、Barrowman の式を手計算で書き下した値と比べる（``crates/aero/src/model.rs``\ 、``matches_hand_barrowman``）。
テストの中に、フィンの法線力係数と圧力中心の式を同じ記号で書き、モデルの出力との相対差が 3 %、圧力中心の差が 0.015 m 以内であることを確かめる。

.. code-block:: rust

   assert!((cna - (cna_n + cna_f)).abs() / (cna_n + cna_f) < 0.03, "cna {cna} vs {}", cna_n + cna_f);
   assert!((x - xcp).abs() < 0.015, "xcp {x} vs {xcp}");

パネル法は、球の表面の圧力係数（ポテンシャル流の厳密解 :math:`C_p=1-\tfrac{9}{4}\sin^2\theta`）、細長い回転楕円体、薄い翼の揚力傾斜（Helmbold の式）、細長い物体の :math:`C_{N\alpha}=2` と比べる（``crates/panel/src/tests.rs``）。
飛翔の計算は、真空中の鉛直飛翔の頂点高度を、推力と質量から手計算した式と比べる（``crates/sim/src/flight.rs``\ 、``vacuum_vertical_flight_matches_analytic``）。
自転する地球の飛翔では、Coriolis 力による西向きのずれを一次近似の積分と比べる（``ecef_coriolis_drifts_west``）。
空力の非定常項は、自由振動の固有振動数と対数減衰率を線形理論と、スピンする機体の速いモードと遅いモードの周波数と減衰を Murphy の理論と比べる（``crates/sim/src/flight_dyn_tests.rs``）。

別のモデルや実装との一致を見るテストも多い。たとえば、パネル法と部品積み上げ法（``sample_rocket_vs_barrowman``、``supersonic_comparison_with_barrowman``）、``rk45`` と細かい刻みの ``rk4``\ （``rk45_matches_rk4``）、
軸対称の係数表と、それを一般の係数表に展開した表の飛翔（``general_table_flight_matches_axisymmetric_table``）がある。
これらは相互比較であり、どちらが正しいかは決まらない。差が許容幅に入っていること、つまり二つの実装が互いに食い違っていないことだけを確かめる。

.. _sec-testing-invariants:

保存量と不変量
~~~~~~~~~~~~~~

運動の途中で変わらないはずの量が、数値積分のあいだ保たれていることを確かめる型である。
解析解が得られない運動でも使える。

トルクのない非対称な剛体（慣性乗積を持つ）を 60 秒回し、角運動量の大きさと運動エネルギーの相対変化が 1e-6 以下であることを、積分法 3 種と姿勢の扱い 2 種の 6 通りで確かめる（``crates/sim/src/flight.rs``\ 、``torque_free_asymmetric_body_conserves_energy_and_momentum``）。

.. code-block:: rust

   assert!((l.norm() / l0 - 1.0).abs() < 1e-6, "{i:?}/{at:?} |L|");
   assert!((e / e0 - 1.0).abs() < 1e-6, "{i:?}/{at:?} energy");

Lie 群の積分法は、正規化をしなくてもクォータニオンのノルムが 1 のまま保たれることを、平らな地球と自転する地球、3 種の積分法のすべての組合せで確かめる（``lie_group_keeps_unit_quaternion_without_normalisation``。``max_quat_norm_error < 1e-12``）。
ほかの例を挙げる。

* 静水圧平衡：温度差を加えた大気の圧力が、:math:`\mathrm{d}p/\mathrm{d}z=-\rho g` を Runge–Kutta 法で積分した値と相対誤差 1e-3 以内で一致する（``crates/aero/src/atmosphere.rs``\ 、``offset_is_hydrostatic``）。
* 閉じた表面：パネルのメッシュは、どの辺も 2 枚のパネルだけに共有される（``mesh_is_watertight``）。
* 対称性：サンプルの形状は 2 つの面について対称であり、フィンを 1 枚欠くか動かした形状はそうでない（``crates/cfd/src/symmetry.rs``）。
* 往復の恒等：回転ベクトルを ``exp`` と ``log`` で往復して元に戻る（``crates/sim/src/lie.rs``\ 、``exp_log_round_trip``）。

.. _sec-testing-order:

収束の次数
~~~~~~~~~~

刻み幅を半分にしたときの誤差の縮み方（収束の次数）が理論どおりであることを確かめる型である。
答えの値が合うだけでは見つからない、式の係数の誤りを見つけられる。

DOP853 は、:math:`y'=y\cos t`\ （厳密解 :math:`e^{\sin t}`）を刻み 8 回と 16 回で解き、誤差の比から次数を求める。8 次の方法なので、次数が 7.5 から 8.5 に入ることを確かめる（``crates/sim/src/dop853.rs``\ 、``fixed_step_convergence_order``）。

.. code-block:: rust

   let order = (e8 / e16).log2();
   assert!(order > 7.5 && order < 8.5, "observed order {order} (e8 = {e8:e}, e16 = {e16:e})");

姿勢の Lie 群積分は、自由な剛体を 20 秒積分し、刻み幅 :math:`h` と :math:`h/2` の姿勢の誤差を、RKMK の DP5 で細かい刻みで解いた解と比べる（``crates/sim/src/lie.rs``\ 、``convergence_order_rk4``、``convergence_order_dp5``）。
4 次の方法は 4±0.3、5 次の方法は 5±0.4 を許容する。実際の値は 4.096 と 4.921 である。:numref:`fig-test-order` は、そのときの誤差と、4 次と 5 次の傾きである。

.. _fig-test-order:

.. figure:: /_generated/plots/test_order.*
   :width: 55%

   Lie 群積分の収束の次数のテストの測定。20 秒後の姿勢の誤差を、刻み幅 0.05 s と 0.025 s で示す。破線は 4 次と 5 次の傾きである。値は ``doc/data/testing/test_order.csv`` による。

``dop853`` の係数が 8 次の条件を満たすこと（``quadrature_order_conditions``）と、係数表の整合（``tableau_is_consistent``、``dopri5_tableau_is_consistent``、``rk4_is_consistent``）も、別のテストが確かめる。

パネル法のメッシュの収束は、次数ではなく変化の大きさだけを見る（``crates/panel/src/tests.rs``\ 、``mesh_convergence``）。
サンプル機体のメッシュを細かくしたときの :math:`C_{N\alpha}` の変化が 5 % 以内であることだけを確かめ、収束の次数は測らない。
CFD のメッシュの収束は、SU2 が必要なのでテストではなく、:ref:`page-vv-cfd-verification` に結果がある。

.. _sec-testing-roundtrip:

往復と読み込みのテスト
~~~~~~~~~~~~~~~~~~~~~~

ファイルや設定を書いて読み直したとき、元と同じになること、壊れた入力が正しいエラーになることを確かめる型である。

係数表は、保存して読み直した値がビットまで一致する（``crates/aero/src/table.rs``\ 、``save_load_is_bit_identical``）。

.. code-block:: rust

   for (x, y) in a.iter().flatten().zip(b.iter().flatten()) {
       assert_eq!(x.to_bits(), y.to_bits());
   }

保存は一時ファイルを経由して置き換えるので、途中で止まっても壊れた表が残らない（``save_leaves_no_temporary_files_and_replaces_atomically``）。
読み込みでは、列の順序が違う見出し、有限でない値、メタデータの欠け、BOM つきのファイル、古い版の列構成を扱うテストがある。
STL は、ASCII 形式と二進形式が同じ三角形になること、80 バイトの見出しが ``solid`` で始まる二進形式を二進として読むこと、途中で切れたファイルを切れていると報告すること、NaN の頂点を三角形の番号つきで拒否することを確かめる（``crates/geom/src/stl.rs``）。

形状の抽出は、サンプル機体の三角形を mm の単位にし、任意に回転して平行移動し、元の単位に戻してから抽出して、設計値を復元できるかを確かめる（``crates/geom/src/extract.rs``\ 、``recovers_sample_rocket_in_arbitrary_orientation``）。

.. code-block:: rust

   assert!(aft.dot(q.mul_vec(Vec3::new(1.0, 0.0, 0.0))) > 0.999);
   assert!(close(g.ref_radius, rk.radius, 0.0005), "radius {}", g.ref_radius);
   assert_eq!(f.count, 4);
   assert!(close(f.span, rk.span, 0.002), "span {}", f.span);

設定ファイルは、有効な例題が読めること、範囲外の値や NaN、旧形式の節、組合せの矛盾が決まったコードのエラーになることを、``crates/cli/src/config.rs`` の 41 件が確かめる（コードを個別に確かめるのは、そのうちの一部である）。
``--set KEY=VALUE`` の書式の誤りは、使い方の誤り（終了ステータス 2）になる（``malformed_set_is_a_usage_error_with_exit_status_2``）。
KML は、書き出したファイルを読み戻して同じ値になることを確かめる（``crates/cli/src/kml.rs``\ 、``loaders_round_trip_written_files``）。

.. _sec-testing-golden:

固定した値（回帰テスト）
~~~~~~~~~~~~~~~~~~~~~~~~

ある時点の計算結果を、許容幅つきで固定し、変更で結果が動いたらテストを失敗させる型である。
正しい値の証明ではなく、意図しない変化を見つけるためのものである。

サンプル機体の既定の飛翔は、頂点高度と着地点を固定する（``crates/cli/src/main.rs``\ 、``sample_default_results_are_unchanged``）。

.. code-block:: rust

   assert!((s.apogee - 4440.48).abs() < 0.05 && (s.apogee_time - 26.10).abs() < 0.005, "{} at {}", s.apogee, s.apogee_time);
   assert!((s.landing_east + 655.57).abs() < 0.05 && (s.landing_north + 7707.70).abs() < 0.05, "{} {}", s.landing_east, s.landing_north);

準定常の空力に切り替えた飛翔は、六成分の項を加える前の結果とビットまで一致する（``crates/sim/src/flight_dyn_tests.rs``\ 、``quasi_steady_switch_reproduces_the_quasi_steady_trajectory_bit_for_bit``）。
頂点高度、着地点の東と北、最大 Mach 数の IEEE 754 のビット列を ``u64`` で書いて比べる。
CFD の SU2 設定ファイルの文字列は、そのハッシュ値を固定する（``crates/cfd/src/su2cfg.rs``\ 、``euler_config_text_is_frozen``）。
設定の文章が変わると既存のキャッシュの計算が無効になるので、変えたことを意識的に確かめさせる意図である。
このほか、SU2 の設定の行そのもの（``euler_half_model_golden_lines``、``rans_two_stage_golden_lines``）を固定したテストがある。

固定した値は、検証の結果ではない。意図して計算を変えたときは、テストの値を書き換える。

.. _sec-testing-codes:

エラーコードとドキュメントの対応
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

ソースとドキュメントの食い違いを、ソースを読んで検出する型である。三つのテストがある。

``crates/cli/tests/error_codes.rs`` は、ワークスペース全体のエラーとワーニングのコード ``IY-<領域><番号>`` を調べる。
ソース（``crates/`` の ``.rs`` と ``.py``、``scripts/`` と ``install.sh``）に現れるコードがすべてドキュメント（``doc/source/errors/<領域>.rst``）にあること、
ドキュメントのコードがすべてソースで使われていること、1 つのコードが 1 か所にしか説明されないこと、領域の文字がファイル名と合うこと、重大度が番号の範囲（001〜499 がエラー、500〜999 がワーニング）と合うことを確かめる。
このテストが確かめるのは、コードの文字列がソースに現れることであり、そのコードのエラーを実際に起こせることではない。
個々のエラーを起こして、メッセージの先頭のコードを確かめるのは、各クレートのテストである。たとえば ``crates/cli/src/config.rs`` の ``config_errors_carry_one_code_each`` は、
存在しない設定ファイルが ``IY-C060``、文法の誤りが ``IY-C061``、未知のキーが ``IY-C062``、範囲の誤りが ``IY-C016`` で始まり、1 つのメッセージにコードが 1 つしかないことを確かめる。

``crates/cli/tests/config_keys_documented.rs`` は、設定ファイルの全キーが設定の章に 1 つずつラベル ``cfg-<表>-<キー>`` を持つことを確かめる。
キーは、設定を表す構造体のフィールド宣言をテストが読んで求める。ラベルのないキー、キーのないラベル、重複したラベルのいずれも失敗になる。
フィールドを読む処理自体が壊れて何も読まなくなる事態は、``key_lists_are_not_empty`` が防ぐ。

``crates/cfd/src/lib.rs`` の ``error_codes_are_documented_and_scripts_are_coded`` は、``cfd`` のコード ``IY-F`` について同じ対応を調べ、Python スクリプトのエラーメッセージにコードが付いていることも確かめる。
インストーラのコード ``IY-I`` は ``scripts/test-install-codes.sh`` が同じ対応を確かめ、さらに ``install.sh --bogus`` が終了ステータス 1 と ``error[IY-I001]`` の行を返すことを確かめる。

.. _sec-testing-random:

乱数を使うテストと再現性
~~~~~~~~~~~~~~~~~~~~~~~~

性質ベースのテストの専用ライブラリ（proptest や quickcheck）は使っていない。
乱数を使うテストは、テストの中に書いた乱数（xorshift）にシードを与えて点を作るか、同じシードの実行が同じ結果になることを確かめる。
乱数と固定の掃引は、次のように使う。

* 参照実装との照合：格子による最近傍の三角形の探索を、全三角形を総当たりした結果と、シード 12345 で作った 300 点で比べる（``crates/cfd/src/fidelity.rs``\ 、``nearest_triangle_matches_brute_force``）。
  サンプルの STL でも、シード 777 の 400 点で同じ照合をする。

  .. code-block:: rust

     let brute = s.iter().map(|t| dist2_point_triangle(p, t)).fold(f64::INFINITY, f64::min).sqrt();
     assert!((g.nearest(p).1 - brute).abs() < 1e-12, "{p:?}");

* 再現性：落下分散のモンテカルロは、シードが同じなら結果が一致し、シードが違えば違う（``crates/sim/src/dispersion.rs``\ 、``seed_reproducibility``）。
  サンプル i の乱数がシードと添字だけで決まることも確かめる。スレッド数を 1、4、7 に変えても結果が変わらない（``thread_count_does_not_change_results``）。
* 統計量：既知の共分散を持つ 4 点から、平均、共分散、誤差楕円の半軸と方位を、閉じた式と比べる（``statistics_of_known_covariance``）。
* 固定の掃引：Mach 数を 0 から 3 まで 0.05 刻みの 61 点に固定し、フィンの法線力が線形理論の適用限界の角度で値と傾きについて連続であることを確かめる。迎角 20°、30°、45°、60°、90° では Mach 数を 0.002 刻みで掃引し、法線力と圧力中心に飛びがないことを確かめる（``fin_normal_force_is_c1_in_alpha_and_continuous_in_mach``）。

.. _sec-testing-double:

擬似の SU2 と外部ツールのテスト
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

CFD の実行制御（``crates/cfd/src/runner.rs`` の 29 件の多く）は、SU2 を使わずに確かめる。
テストは、設定ファイルから ``MACH_NUMBER``、``AOA``、``RESTART_SOL`` を読み、擬似の ``history.csv`` を書く shell スクリプトを SU2 の代わりに実行する。
スクリプトの動作は、同じ場所の ``mode`` ファイルで切り替える。

.. csv-table:: 擬似の SU2 のモード
   :delim: ¦
   :header-rows: 1
   :widths: 22 78

   モード¦動作
   ``ok``¦残差が 1 行に 0.25 桁ずつ下がり、25 行で終わる
   ``fail``¦標準エラー出力に ``boom`` を書き、終了ステータス 1 で終わる
   ``stall``¦残差が 1 行に 0.02 桁しか下がらない
   ``unstable``¦力の係数が 1 行おきに振れる
   ``slow``、``trend``、``trendlimit``、``eflip``¦履歴を 1 行ずつ、またはまとめて書き、ランナーが途中で止める判断を確かめる

これで、ケースの行列を順に解くこと、直前のケースの解から温かい再開をすること、失敗を記録して全体は止めないこと、再開時に終わったケースを飛ばして設定が変わったケースだけやり直すこと、
収束の判定、発散したときの保守的な設定での再試行、履歴のない実行を失敗とすること（``missing_history_is_a_failure``）を確かめる。
たとえば ``runs_matrix_with_warm_starts_and_progress`` は、擬似の SU2 が係数を 0.05 :math:`\alpha` のように返すので、6 ケースの結果が手計算の値と一致し、冷えた開始が 1 ケースだけで残りが温かい開始であることを確かめる。
この型のテストは Unix の shell と ``bc`` を使う。

実際のソルバを走らせるテストは、``crates/cfd/tests`` の 16 件で、すべて ``#[ignore]`` である。
``real_su2.rs`` の 9 件は、たとえば半頂角 10° の円すいと円柱の Mach 2 の圧力係数を Taylor–Maccoll の値と 10 % 以内で比べ（``cone_cp_matches_taylor_maccoll``）、
RANS のプリズム層のメッシュが有効であること、STL の和集合が種数 0 で、メッシュを切れることを確かめる。
``real_step.rs`` の 7 件は、CAD の B-rep（STEP）の壁面を使う。サンプルを全領域でメッシュし、SU2 が読めること、単位を誤った STEP を位置合わせが検出すること、対称な形状が 1/4 領域と 1/2 領域になることを確かめる。
SU2 を使うケースには、所要時間が 5 分や 30 分と注記されたものがある。
``#[ignore]`` は 18 件で、残りの 2 件は計時（``panel`` の ``default_table_timing``）と積分法の比較表の出力（``cli`` の ``integrator_matrix_report``）である。

.. _sec-testing-cli-e2e:

コマンドの通しのテスト
~~~~~~~~~~~~~~~~~~~~~~

``crates/cli/src/main.rs`` の 76 件は、``ignisyeet`` の処理を関数として呼び出して確かめる。
実行ファイルを起動するテストはなく、``prepare``、``simulation``、``Cli::try_parse_from`` を同じプロセスで呼ぶ。
例題の設定（``examples/sample.toml``）を読み、形状の抽出、係数表の作成、飛翔までを実際に計算するものがある。

* 例題の飛翔が着地し、外挿した風についてのワーニングを出す（``sample_flight_lands_and_warns_about_the_extrapolated_wind``）。
* CAD の機体から、同じ機体を STL に書き出したものと同じ形状と部品積み上げ法の表ができる（``a_cad_rocket_gives_the_geometry_and_barrowman_table_of_its_stl``）。
* 設定の同一性（ハッシュ）は、設定が変わると変わり、出力先と計算資源の指定では変わらない（``run_hash_follows_the_configuration_but_not_output_or_resources``）。係数表を使い回す条件もここで確かめる。
* 失敗した飛翔は、古い結果を残さず、失敗の印を書く（``a_failed_sim_leaves_a_failure_marker_not_the_old_results``）。
* ``--set`` と ``--out`` は、値を設定と同じ規則で検査し、ハッシュに入る（``overrides_enter_the_run_hash_but_out_does_not``）。

実際の ``ignisyeet`` コマンドを起動する確認は、CI の smoke test が受け持つ。
そこでは、例題で ``geom``、``aero``、``sim``、``dispersion`` を実行し、作図を行い、不完全な飛翔の終了ステータスが 3 であることを確かめる。

.. _sec-testing-margins:

判定幅と実際の偏差
~~~~~~~~~~~~~~~~~~

テストの許容幅は、実際の偏差よりずっと広いことが多い。
:numref:`fig-test-margins` は、値を表示するテストについて、測った偏差を許容幅で割った値を示す。
1 より小さければテストに通り、1 に近いほど余裕がない。

.. _fig-test-margins:

.. figure:: /_generated/plots/test_margins.*
   :width: 85%

   テストが測った偏差を、そのテストの許容幅で割った値（対数軸）。値はテストを ``--nocapture`` つきで実行した出力であり、出所は ``doc/data/testing/test_margins.csv`` の先頭にある。

読み取れることは次のとおりである。

* 保存量（緑）のうち、自由な剛体のエネルギーと角運動量は、許容幅 1e-6 に対して 1e-11 台で、許容幅の 5〜6 桁下にある。非対称剛体の飛翔は 1e-8 台で、許容幅の 2 % から 4 % である。
  クォータニオンのノルムのずれ（5.77e-15）は、許容幅 1e-14 の 58 % であり、倍精度の丸めの大きさそのものである。
* 解析解との比較（青）は、許容幅の 1 % から 65 % である。円すいの圧力係数は、許容幅の 65 % を使っている。
* 他のモデルとの相互比較（橙）は、パネル法と部品積み上げ法の :math:`C_{N\alpha}` が 26 % 違い、許容幅 30 % のほぼ上限である。
  このテストは、2 つの手法の差が 30 % を超えて広がらないことだけを確かめる。差の大きさは、フィンと胴体の干渉の扱いの違い（Barrowman の係数 1.33 に対し、パネル法は約 1.6）によるものであり、テストのコメントに理由が書いてある。
* 許容幅が実際の偏差よりずっと広いテストは、許容幅の内側の変化を検出しない。

.. _sec-testing-crates:

クレートごとのテストの内容
--------------------------

各節は、モジュールごとのテスト関数の数と、確かめる内容である。数は ``cargo test -- --list`` の出力による。

.. _sec-testing-geom:

``geom``\ （24 件）
~~~~~~~~~~~~~~~~~~~

.. csv-table:: ``geom`` のテスト
   :delim: ¦
   :header-rows: 1
   :widths: 20 8 72

   モジュール¦件数¦確かめる内容
   ``extract``¦14¦機軸の向き（右手系、ノーズ側）、サンプル機体の寸法の復元（任意の回転と平行移動）、四角いフィンと前進角のフィン、円すいノーズの接合角、半球ノーズとボートテール、ランチラグと突き出したノズルへの耐性、向きの自動反転、開いた表面の拒否、警告（寸詰まり、フィンが底面より後ろ）
   ``stl``¦6¦ASCII と二進形式の一致、``solid`` で始まる見出しの二進形式、途切れたファイル、NaN の頂点、エラーコードが 1 つずつ付くこと、読み込みエラーがファイル名を示すこと
   ``math``¦4¦3×3 行列の固有値分解、逆行列と積、クォータニオンと回転行列の往復

.. _sec-testing-aero:

``aero``\ （58 件）
~~~~~~~~~~~~~~~~~~~

.. csv-table:: ``aero`` のテスト
   :delim: ¦
   :header-rows: 1
   :widths: 20 8 72

   モジュール¦件数¦確かめる内容
   ``atmosphere``¦5¦標準大気の値、温度差 0 で標準大気と一致、温度差つきの静水圧平衡、一定大気、非現実的な設定の拒否
   ``model``¦19¦Barrowman の式の手計算との一致、Mach 数について連続（遷音速）、フィンの法線力が大迎角で有界で 1 階連続、横流れの混合関数が単調で端に一致、ロール項の符号と大きさ、非定常項の遅れ和、見かけ質量、Basic Finner の超音速の :math:`C_{N\alpha}` が線形理論の積み上げと 1 % 以内で一致
   ``table``¦34¦内挿と外挿（線形か打ち切り）、迎角の外挿の上限と回数、保存と読み込み（ビットまで一致、一時ファイル）、不正な見出し・非有限値・メタデータ・BOM・古い版の読み込み、軸対称の表の一般の表への展開と静的余裕、誘導ロール、迎角の格子が 0 を通ること

.. _sec-testing-panel:

``panel``\ （36 件、``#[ignore]`` 1 件）
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

.. csv-table:: ``panel`` のテスト
   :delim: ¦
   :header-rows: 1
   :widths: 20 8 72

   モジュール¦件数¦確かめる内容
   ``tests``\ （``tests.rs``）¦24¦球と回転楕円体の表面の圧力係数、細長い物体の :math:`C_{N\alpha}`、閉じたメッシュ、薄い翼と Helmbold の式、メッシュの収束、遷音速の連続性と帯の端、フィンの横流れと上昇流、メモリの見積り、``#[ignore]`` の計時の 1 件
   ``supersonic``¦5¦円すいの圧力係数、斜め衝撃波と Prandtl–Meyer、衝撃波が離れる角度の前後で圧力係数が連続
   ``influence``¦3¦影響係数の解析的な式と数値積分との一致、ダブレットの表裏、遠方場の厳密解
   ``viscous``¦2¦平板の摩擦係数（層流は :math:`0.664/\sqrt{Re}`、乱流は :math:`0.0592\,Re^{-0.2}`）、設定された大気を使うこと
   ``memory``¦2¦``/proc/meminfo`` と cgroup のファイルの読み取り、制限が正の値

.. _sec-testing-sim:

``sim``\ （137 件）
~~~~~~~~~~~~~~~~~~~

.. csv-table:: ``sim`` のテスト
   :delim: ¦
   :header-rows: 1
   :widths: 26 8 66

   モジュール¦件数¦確かめる内容
   ``flight``¦50¦真空の飛翔の解析解、頂点とレール離脱の補間（Hermite）、着地の補間、平らな地球と ECEF の一致と Coriolis、積分法 3 種と姿勢の扱いの全組合せ、保存量、慣性、推力のずれ、パラシュートと二段開傘、発散の検出、最大時間
   ``flight``\ （``dyn_tests``）¦12¦自由振動の線形理論、Murphy の理論、Magnus 力と回転誘導力の向き、横風のせん断での遅れ、遅れの減衰、準定常のスイッチがビットまで一致、ロール共振の報告
   ``dispersion``¦30¦シードと再現性、スレッド数への非依存、既知の共分散、百分位数の補間、落ちなかった標本の扱い、ばらつきの乱数の打ち切り、レールの傾き、未知のキーの拒否
   ``env``¦21¦風のモデル（べき則、対数則、プロファイル、地面の近くの接続）の値と検査、:math:`J_2` 重力の大きさと向き
   ``lie``¦8¦指数写像と対数写像、微分の数値との一致、RKMK の収束の次数、正規化つきの RK4 との比較
   ``dop853``¦6¦係数表、次数の条件、収束の次数、適応刻み、Arenstorf 軌道
   ``motor``¦4¦推力曲線のファイルの読み取りと積分、不正なファイルの拒否
   ``frame``¦2¦ECEF の座標系の幾何、重力の水平成分の残りが 0
   ``integrator``、``tableau``、``geo``¦2、1、1¦RK4 と Dormand–Prince の収束、RK4 の係数表の整合、緯度経度高度と ECEF の往復

.. _sec-testing-cfd:

``cfd``\ （181 件、``#[ignore]`` 16 件）
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

.. csv-table:: ``cfd`` のテスト
   :delim: ¦
   :header-rows: 1
   :widths: 24 8 68

   モジュール¦件数¦確かめる内容
   ``runner``¦29¦擬似の SU2 による実行制御（前節）。収束、止める判断、再開、RANS の受け入れ規則と再試行
   ``surface``¦14¦STL の修復と向き、対称面での切り取り、対称なフェアリング、開いた表面の拒否、パネルのメッシュの書き出し
   ``config``¦12¦設定の既定値、不正な格子の拒否、未知のフィールドの拒否、RANS の既定値、メッセージの先頭にコードが付くこと
   ``mesh``¦12¦境界層の最初の高さと厚さ、パラメータとスクリプトの一致、プリズム層、設定の変更でハッシュが変わること、中断したメッシャの報告
   ``forces``、``table``¦11、11¦六成分の対応と鏡像、圧力中心、粗さと底面の増分、人工の力の場から係数表を作る
   ``step_mesh``¦11¦STEP と IGES の単位、STL との位置合わせ、内外判定、フィンの細分化、ハッシュ
   ``su2cfg``、``fidelity``、``symmetry``¦6、6、7¦SU2 の設定の固定、表面の距離（総当たりとの一致）、対称性の判定と決定
   ``general``、``history``、``tools``¦8、7、9¦鏡像で象限を補う、履歴の読み取りと収束と発散の判定、ツールの検出とバージョンの警告
   ``case``、``asymmetry``、``cad_input``、``stl_mesh``、``lib``¦4、3、3、2、10¦ケースの順序とハッシュ、非対称の補正、CAD の入力、STL の底面の扱い、資源の割り当て、エラーコードの対応

.. _sec-testing-cli:

``cli``\ （157 件、``#[ignore]`` 1 件）
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

.. csv-table:: ``cli`` のテスト
   :delim: ¦
   :header-rows: 1
   :widths: 24 8 68

   モジュール¦件数¦確かめる内容
   ``main``¦78¦コマンドの通し（前節）、警告の条件（静安定、レール離脱、風、表の範囲）、使えない結果の判定、キャッシュのキー、``summary.json``、``--set`` と ``--out``
   ``config``¦41¦設定の検査（範囲、組合せ、``--set`` の上書き）、CAD の機体、RANS の既定値、エラーコード
   ``kml``、``report``¦11、11¦KML の構造と色、誤差楕円、レポートの節と数値、エスケープ
   ``report_svg``、``view``、``ui``、``outfile``¦5、4、2、2¦SVG の目盛りと間引き、端末の表示、メッセージの表示、書き込みが途中で壊れないことと実行中のロック
   ``tests/``\ （結合）¦3¦設定キーとエラーコードのドキュメントの対応

.. _sec-testing-python:

Python と shell のテスト
~~~~~~~~~~~~~~~~~~~~~~~~

``python/test_plot.py`` は 41 件で、``unittest`` を使う。
作図の元になる量の計算（一般の係数表からの合計と傾き、静安定余裕、高度の基準）、失敗した計算の扱い、スキップした図の古い PNG の削除、出力が古いことの検出、使えない結果の帯の表示を確かめる。
たとえば、一般の係数表から :math:`C_D`、:math:`C_{N\alpha}`、:math:`x_{cp}` の合計を作る処理は、成分に定数を入れた人工の表を作り、合計が定数どおりになることを ``assert_allclose`` で確かめる。
図の見た目は確かめない。作図関数が例外なく図を出し、必要な要素（凡例、帯、軸）を含むかどうかだけを見る。

``scripts/test-install-codes.sh`` は、``install.sh`` の ``die``\ 、``warn`` がすべてコードを持つこと、コードがドキュメントにあり、ドキュメントのコードが使われていることを確かめる。
さらに ``install.sh --bogus`` と ``--version 1.2`` を実際に実行して、終了ステータスとメッセージの形を確かめる。

.. _sec-testing-limits:

テストが確かめていないこと
--------------------------

テストの範囲を、誤解しないために挙げる。

* 実測との比較（妥当性確認）はテストにない。Basic Finner の実測との比較は、``doc/scripts/`` のスクリプトと ``doc/data/validation/`` の値で行い、:ref:`page-vv-validation` に述べる。
  ``basic_finner_supersonic_cna_is_the_linear_theory_build_up`` は Basic Finner の形状を使うが、実測値ではなく線形理論の式と比べる（実測との 11〜13 % の差は、テストの注釈にも書いてある）。
* CFD のソルバを実際に走らせるテストは ``#[ignore]`` であり、CI では実行されない。CI が確かめる CFD は、擬似の SU2 による実行制御と、設定ファイルの文字列、メッシュのパラメータ、結果の後処理である。SU2 の出力の値そのものを CI は検査しない。
* パネル法、CFD、部品積み上げ法の結果の差は、許容幅の中にあることだけを確かめる（相互比較）。どの手法が実機に近いかは決まらない。
* 収束の次数のテストは積分法だけで、メッシュの収束の次数を測るテストはない。
* エラーコードの対応は、コードの文字列がソースにあることの検査であり、各コードのエラーを起こせることは、一部のコードについて個別のテストが確かめるだけである。
* 設定キーの対応は、構造体のフィールド宣言を文字列として読んで求める。読み方が壊れた場合の保護は、``key_lists_are_not_empty`` による最小数の検査だけである。
* 実際の ``ignisyeet`` の実行ファイルを起動するテストはなく、起動は CI の smoke test が受け持つ。終了ステータス 1（エラー）を確かめるテストはなく、2（使い方の誤り）は引数の解析（``Cli::try_parse_from``）だけを確かめる。
* 実機の飛翔データとの比較と風の実況との比較は、テストにない。固定した値（回帰テスト）は、リリースのビルドで 3 つの環境（Linux x86_64、macOS の Intel と Apple Silicon）の同じ値に対して実行される。
  ただし、固定した値は許容幅つきなので、環境による最後の桁の違いは許容される。固定したビット列と比べるのは、準定常の切り替えのテストの 1 件だけである。
* テストの数は、網羅の程度を表さない。行や分岐の網羅率は測っていない。

テストの追加や変更の手順は :ref:`sec-dev-build` にある。
