検証
====

モデルの式が正しく実装されていることを、解析解や手計算と比較して確かめている。
以下の検証はすべて自動テストになっており、次のコマンドで再現できる。

.. code-block:: sh

   cargo test --release

.. list-table:: 自動テストの一覧
   :header-rows: 1
   :widths: 34 66
   :class: longtable

   * - テスト
     - 内容
   * - ``geom::extract``
     - 既知の寸法のサンプル機体から形状を復元できるか（任意の回転・平行移動・単位換算を加えて）、機体軸 :math:`(x_b,y_b,z_b)` が右手系でノーズ方向を向くか
   * - ``geom::math``
     - 対称行列の固有値分解、クォータニオンと回転行列の相互変換、3×3 行列の逆行列・積・転置・行列式
   * - ``geom::stl``
     - ASCII 形式とバイナリ形式の STL が同じ三角形を与えるか。見出しが ``solid`` で始まり ``facet`` を含むバイナリ形式をバイナリとして読み、末尾に改行のない ASCII 形式を ASCII として読むか（``binary_stl_with_a_solid_header_is_read_as_binary``）
   * - ``aero::atmosphere``
     - 標準大気の代表値、温度差 0 での標準大気との一致、温度差を加えた大気の静水圧平衡、一定大気
   * - ``aero::model``、``panel``、``aero::table``
     - 部品積み上げ法、パネル法、係数表の検証（解析解、手計算、連続性、展開、保存と読み込み）。内容は :ref:`sec-vaero-barrowman`、:ref:`sec-vaero-table`、:ref:`sec-vaero-panel` にまとめる
   * - ``sim::motor``
     - 推力曲線の読み込み、力積の積分、推進剤の消費、燃焼時間・推力の伸縮（モンテカルロ用）、:math:`t=0` の推力、BOM、推力曲線の警告、推力の終わりの時刻（末尾の推力 0 の点を数えない）
   * - ``sim::env``
     - 風モデル（一定・べき法則・対数則・高度別の表）、地面付近の接続（連続性・傾き・上空で不変・非負で単調・設定の検証）、対数則の粗度長が基準高度の半分より小さいこと、表の検証（負の高度の行の拒否を含む）、:math:`J_2` 重力の大きさと向き、平面地球での :math:`J_2` の拒否
   * - ``sim::frame``
     - ECEF 座標系の幾何（高さ、ENU の往復、重力の向きと大きさ、遠心加速度、Coriolis 加速度の向き）。有効重力（``constant`` と ``inverse_square``）では、万有引力に遠心項を加えた正味の重力が有効重力に等しく、高度 0、2、20、80 km で水平成分が東・北とも :math:`2\times10^{-4}\ \mathrm{m/s^2}` 未満であること（``ecef_net_gravity_has_no_horizontal_residual``）
   * - ``sim::integrator``
     - 調和振動子による ``rk4`` と ``rk45`` の収束
   * - ``sim::dop853``
     - 係数表の整合（段数・:math:`c_i` と行和・求積の次数条件）、固定刻みでの収束次数、調和振動子と Arenstorf 軌道の適応積分、刻み幅の倍率の上下限と単調性
   * - ``sim::lie``
     - 指数写像と対数写像の往復、:math:`\mathrm{dexp}` と逆写像の数値微分との一致、自由剛体での収束次数・ノルム・保存量、通常の方法との比較、埋め込み誤差の合成
   * - ``sim::dispersion``
     - 乱数の再現性、標準偏差 0 での公称飛翔との一致、風向のばらつき、共分散・誤差楕円、失敗サンプル、ばらつきの反映、
       ロールと非対称の標準偏差が最初の 11 項目の乱数を動かさないことと、6 項目の反映、空力の 3 項目（``lag_scale``、``magnus_scale``、``cmq_scale``）の反映。
       パーセンタイルの線形補間（``percentiles_interpolate``）、着地しないサンプルが統計を不完全にし最大着地距離を下限にすること（``non_landing_samples_make_statistics_incomplete``）、
       短い ``max_time`` で着地なしの要約になること（``short_max_time_gives_unlanded_summaries``）、着地したサンプルが 1000 未満では P99.9 が ``null`` になること、6 項目の標準偏差の検査（``new_monte_carlo_sigmas_are_validated``）、
       レールの傾きの回転（``rail_tilt_rotates_the_axis``）、鉛直のレールでの等方性（``rail_tilt_spread_is_isotropic_for_a_vertical_rail``）、
       範囲外の射角のサンプルの失敗（``rail_elevation_outside_the_valid_range_fails_the_sample``）、すべての降下モードの静安定余裕（``every_dispersion_run_reports_its_stability``）、
       既定の乱数が変わらないこと（200 サンプルの和の固定値との比較）と、``truncate_sigma`` が標準正規乱数を切り詰め、内側の値を変えないこと（``default_draws_are_unchanged_and_truncation_clips_the_standard_normal``）、P99.9 の補間、
       0・負・非数・無限大の ``truncate_sigma`` をモンテカルロ解析の入口で拒み、与えた値を ``summarize`` が記録すること（``monte_carlo_validates_and_reports_truncate_sigma``）
   * - ``ignisyeet``\ （``cli``）
     - 設定ファイルの読み込み（ロール・慣性テンソル・推力の傾き・``table_kind`` の項目を含む）、未知のセクションの拒否、モード間の整合性、外部係数表の読み込み、
       ロール項のない外部表への補完、遅れと Magnus の列のない外部表とパネル法・CFD の表への遅れの和の補完（:ref:`sec-verify-unsteady`）、``sim.quasi_steady``、ロール共振の警告と結果パネルの行、
       一般表でのサンプルの頂点高度、カントによるロールとジオメトリへの反映、
       モンテカルロ CSV の列数。
       範囲外と有限でない数値の拒否（``rejects_out_of_range_and_non_finite_numbers``、``rejects_bad_motor_and_monte_carlo_values``）、既定の ``max_time`` が 3600 s であること（``default_max_time_is_one_hour``）、
       キャッシュキーが使わないブロックと CFD の資源の項目を無視し、モデルの版を含むこと（``cache_key_ignores_unused_blocks_and_cfd_resources_but_not_the_model``、``model_version_is_part_of_the_cache_key``）、
       設定のハッシュが出力先と資源に依存しないこと（``run_hash_follows_the_configuration_but_not_output_or_resources``）、
       静安定余裕・風モデルの外挿・着地しないことの警告（``stability_wind_and_landing_warnings``）、大きさの警告と機体外の重心（``implausible_size_and_cg_outside_the_body``）、
       ``summary.json`` の着地・ハッシュ・警告（``summary_json_records_landing_hash_and_warnings``）、風の格子の最大着地距離の表示（``wind_grid_line_marks_failures``）、
       サンプルの飛翔が着地して警告が一つであること（``sample_flight_lands_and_warns_about_the_extrapolated_wind``）、
       KML が古い出力と着地しなかったケースを除くこと（``stale_outputs_are_skipped``、``unlanded_wind_grid_rows_are_left_out``）、
       静的に不安定なサンプルを降下モードごとに数えて終了ステータス 3 にすること（``unstable_samples_are_counted_per_mode_and_make_the_run_unusable``）、CSV の ``stable`` と ``broadside_time`` の列（``dispersion_csv_files_carry_stable_and_broadside_columns``）、
       横向きに落ちた弾道落下の警告と数（10 s 以下とパラシュート降下は数えない。``broadside_ballistic_falls_are_warned_and_counted``）、
       しきい値を破った側に読める桁数での数値の表示（``threshold_values_print_on_the_violated_side``）、
       形の誤った ``--set`` が終了ステータス 2 の使い方の誤りになること（``malformed_set_is_a_usage_error_with_exit_status_2``）、``sim.dt`` の下限と ``rk4`` のステップ数の上限（``dt_has_a_lower_bound_and_a_step_count_limit``）、
       範囲外の整数の ``--set`` のエラー（``set_shape_is_checked_by_the_parser_and_huge_integers_are_named``）、
       Mach 数の範囲外の引き出しが ``RESULTS NOT USABLE`` の文と ``machs_in_range = false`` になること（``aero_range_warning_and_json``）、
       ``dispersion`` が、実行の最初に同じモードの前回の結果だけを消すこと（``a_dispersion_run_removes_the_earlier_results_of_its_mode``）、
       ``profile`` の風の ``wind_at_ref_height`` が ``wind.ref_height`` での表の風に等しく、ほかの風モデルでは書かれないこと（``profile_targets_fly_like_the_wind_grid_case`` の中の確認）、
       出力ファイルの書き込みが一時ファイルを残さないこと、動いている実行の印のある出力先を拒み、止まった実行の印を消すこと（``write_is_atomic_and_leaves_no_temp_file``、``live_lock_is_refused_and_stale_lock_is_cleaned``、``output_directory_of_a_live_run_is_refused``）、
       実行のハッシュが外部の係数表の付属 JSON を含むこと（``run_hash_covers_the_aero_table_sidecar``）、既定のモータ位置の検査（``default_motor_position_is_checked_against_the_body``）、
       ランチャ離脱の風による迎角の警告が、風の格子の最大の風速とモンテカルロの公称の風速 + 2 :math:`\sigma` を使うこと（``rail_exit_wind_warning_uses_the_reference_speed_of_the_study``）、
       失敗した ``sim`` が古い結果の代わりに失敗の印を残すこと（``a_failed_sim_leaves_a_failure_marker_not_the_old_results``）、着地したサンプルが 1 つのときの誤差楕円の表示（``monte_carlo_panel_with_one_landed_sample_has_no_nan_ellipses``）、
       一般表の展開を数える格子の上限（``general_table_expansion_counts_for_every_method_that_expands``）、KML の説明欄の CDATA を二重にエスケープしないこと、
       ``truncate_sigma`` の読み込みと検査（``monte_carlo_truncation_key``）、``aero.panel`` の検査と ``transonic[1]`` の上限（``panel_options_are_validated``）、
       格子の上限が片側の迎角の格子を数えること（``aero_grid_cap_counts_the_one_sided_alpha_grid``）、``[output]`` の ``dir`` の既定値と 0 サンプルの拒否（``output_without_dir_and_zero_monte_carlo_samples``）、
       パネル法と CAD 面の CFD でフィンの厚さが正であること（``panel_and_cad_cfd_need_fins_with_thickness``）、機体より長いモータのエラー（``default_motor_position_is_checked_against_the_body``）、
       ランチャ離脱の風の警告が風のプロファイルを基準の風速に拡大すること（``rail_exit_wind_warning_scales_a_wind_profile_to_the_reference_speed``）、
       Makefile が字下げ、引用符付きのキー、空白を含む見出しの ``[output] dir`` を読むこと（``makefile_reads_indented_and_quoted_output_dirs``。``make`` がない環境では省く）
   * - ``python/test_plot.py``
     - 作図の見出しの帯と印（``uv run --project python python -m unittest discover -s python`` で実行する）。問題のない落下分散に帯がないこと、
       不安定・判定できない・迎角の範囲外の結果が ``RESULTS NOT USABLE``、失敗と着地しないケースが ``INCOMPLETE``、横向きの落下が ``CAUTION`` になること、
       風の格子とモンテカルロの図で不安定、判定できない、横向きに落ちた着地点に印が付き、飛翔経路の図に帯が出ること（``Usability`` の各テスト）、
       モンテカルロの図に P99.9 が出ることと、P99.9 のない統計では ``(P99.9 needs ≥ 1000 landed samples)`` と添えること、風の格子の線が横向きのケースで切れて印は残ること、飛翔経路の図の余裕の欄がランチャを離れたあとの圧力中心と重心の距離であること、
       一般表の :math:`C_{N\alpha}` と :math:`x_{cp}` が迎角 0 の前後の符号つきの中心差分（負の迎角がなければ片側差分）で求まり、求まらない値が NaN になること、
       失敗した ``sim`` の印があっても落下分散の結果どうしの設定の違いを警告すること、結果のない落下分散の古い PNG を消すこと
   * - ``sim::geo``
     - 測地座標と ECEF の往復変換、ENU の北方向 1 km の緯度差
   * - ``sim::flight``
     - 真空中の鉛直飛翔と解析解の比較、横風に対する風見効果と流され、ECEF と平面地球の一致、Coriolis 力による西向きのずれ、``rk45`` と ``rk4`` の一致、積分法と姿勢の更新法の全組合せの一致、``dop853`` の評価回数、厳しい許容誤差での着地、
       ロール（定常ロール角速度、カントの基準）、慣性テンソル（符号、乾燥重心の横ずれ、トルクなしの回転）、推力の傾きとノズルのオフセット、一般表の飛翔、ランチャ上のロール角、二段開傘と頂点の補間と発散の検出（:ref:`sec-verify-dual`）、
       ステップ内の事象（ランチャ離脱からの再開、同じステップの着地と開傘、``max_time``、推力曲線の折れ点。:ref:`sec-verify-events`）、
       小迎角の静安定余裕と最小の位置、横向きの落下の時間、燃焼中の開傘の延期、着地の状態を含む最大値（:ref:`sec-verify-margin`）、
       見かけの質量の遅れと対流の回転、細長い物体の理論のピッチ減衰との比較、回転による法線力、Magnus 力とモーメント、誘導ロール、ロール共振（:ref:`sec-verify-unsteady`）

形状抽出
--------

サンプル機体の STL を mm 単位に拡大し、任意の回転と平行移動を加えたうえで抽出しても、
:numref:`tbl-geometry` の精度（フィン寸法の誤差 1 mm 以内）で設計値を復元できる。
ノーズの向き（どちらの端がノーズか）も正しく判定される。円錐ノーズ・3 枚フィン・後端から離れたフィンの組合せでも、
接合部の半頂角 :math:`\arctan(R/L_n)`、体積比 1/3、フィンの位置を正しく求められることを確かめている。
向きの判定と警告については、次のテストで確かめている。

* ``hemispherical_nose_with_long_boattail_is_oriented_correctly``：半球ノーズと長いボートテールの機体（端の半径だけでは向きが決まりにくい形）を、メッシュの向きにかかわらず正しい向きに抽出し、警告が出ない。
* ``reversed_sample_rocket_is_flipped_automatically``：逆向きのサンプル機体を自動で反転する。
* ``launch_lug_does_not_disturb_nose_or_fins``：ランチラグ（短い突起）が基準半径、ノーズ、フィンの検出に影響しない。
* ``protruding_nozzle_keeps_nose_detection_and_warns``：突き出したノズルがあってもノーズを検出でき、底面半径の警告が出る。
* ``plausible_rocket_has_no_warnings_and_odd_proportions_warn``：標準的な機体には警告がなく、寸詰まりの機体には全長と直径の比の警告が出る。
* ``explicit_nose_direction_is_respected_and_forward_fins_warn``：``nose_direction`` の指定は覆されず、フィンが前半にあれば警告が出る。
* ``open_surfaces_are_rejected_and_small_defects_warned``：サンプル機体の閉じた表面には開いた辺も非多様体の辺もない。三角形 1 枚の面は ``the STL surface is not closed`` のエラーになり、三角形を 1 枚除いた表面は 3 本の開いた辺の警告、三角形を 1 枚重ねた表面は 3 本の非多様体の辺の警告になる。
  開いた辺が 1 % 未満でも、機体の半分だけの表面、底面を除いた表面（``a hole bounded by … open edges is 0.100 m across (100 % of the body diameter 0.100 m)``）、フィンの平らな側面を 1 枚除いた表面は、穴の幅の判定でエラーになる。
* ``fins_behind_the_body_base_warn``：サンプル機体にはフィンの後縁の警告が出ない。根元の後縁または後退した翼端が機体の後端より後ろにあるフィンは ``the fin trailing edge at … lies aft of the body end at …`` の警告になり、フィンが底面より後ろへ突き出したメッシュは ``the aft end of the mesh is wider than the body`` の警告になる。

空力モデル
----------

空力モデル（部品積み上げ法、パネル法、CFD、係数表）の検証と、手法どうしの相互比較、計算時間の測定結果は、:doc:`verification_aero` にまとめた（:ref:`sec-vaero-barrowman`、:ref:`sec-vaero-table`、:ref:`sec-vaero-panel`、:ref:`sec-vaero-cfd`、:ref:`sec-vaero-compare`、:ref:`sec-vaero-timing`）。
解析解や手計算との比較、メッシュの収束、``doc/data/cfd/`` に保存した CFD の計算結果もそちらにある。

標準大気
--------

海面で :math:`\rho=1.225\ \mathrm{kg/m^3}`, :math:`a=340.29\ \mathrm{m/s}`、11 km で :math:`T=216.77\ \mathrm{K}`、
20 km で :math:`\rho=0.0889\ \mathrm{kg/m^3}` となることを確かめている（いずれも文献値 :cite:`us76`）。

.. _sec-verify-env:

温度差を加えた大気・一定大気
~~~~~~~~~~~~~~~~~~~~~~~~~~~~

* **温度差 0 での一致**：``temperature_offset = 0`` の大気は、−500 m から 80 km までの 10 点で標準大気と **完全に** 同じ値を返す。
  :eq:`eq-us76-offset` を :math:`\Delta T=0` で評価した圧力も、標準大気の圧力と相対誤差 :math:`2\times10^{-5}` 以内で一致する。
* **静水圧平衡**：:math:`\Delta T=15` K で、高度 0, 3, 11, 18, 30, 50 km の圧力が、微分方程式
  :math:`\mathrm{d}p/\mathrm{d}z=-\rho\,g(z)`\ （:math:`g(z)=g_0(R_0/(R_0+z))^2`、:math:`\rho=p/(RT)`）を海面から 4 次 Runge–Kutta 法
  （20 000 刻み）で積分した値と相対誤差 :math:`10^{-3}` 以内で一致することを確かめている（差は 11 km で :math:`2\times10^{-15}`、50 km でも :math:`10^{-10}` 程度）。
  温度は標準大気より厳密に 15 K 高く、海面の密度比は 0.9505、同じ高度で空気は標準より薄い。
* **一定大気**：どの高度でも与えた :math:`\rho,a,\mu` を返し、:math:`p=\rho RT`、:math:`a=\sqrt{\gamma RT}` が成り立つ。0 以下の値は拒否する。

重力と風
--------

重力（:math:`J_2`）
~~~~~~~~~~~~~~~~~~~

:eq:`eq-j2` の大きさを、海面の赤道と極で確かめている。

.. list-table:: :math:`J_2` 重力の地表での大きさ（万有引力のみ）
   :header-rows: 1
   :widths: 30 30 40

   * - 位置
     - 計算値 [m/s\ :sup:`2`]
     - テストの許容範囲
   * - 赤道
     - 9.8142
     - 9.814 ± 0.002
   * - 極
     - 9.8321
     - 9.832 ± 0.01

赤道では万有引力が地球中心を向き、極では :math:`x,y` 成分が 0 になる（ともに数値の丸めの範囲）。
また、地心緯度 45° の点では、赤道向きに :math:`-0.01` から :math:`-0.02\ \mathrm{m/s^2}` の成分を持つ
（:math:`3J_2g\sin\phi\cos\phi\,(a/r)^2` に相当）。平面地球モードで ``gravity = "j2"`` を指定するとエラーになり、ECEF モードでは受け付ける。

風モデル
~~~~~~~~

* **べき法則・一定風**：基準高度で :math:`w_{\mathrm{ref}}` に一致し、128 m（基準高度の 64 倍）では 1.9 倍を超える。一定風は高度によらず一定である。
* **対数則**：:math:`h_{\mathrm{ref}}=10` m、:math:`z_0=0.1` m、:math:`w_{\mathrm{ref}}=4` m/s で、基準高度で 4 m/s、:math:`h\le z_0` で 0、
  100 m で :math:`4\ln1000/\ln100=6` m/s となる。:math:`2z_0\ge h_{\mathrm{ref}}` と負の基準高度はエラーになる（風モデルを直接作るライブラリの呼び出しでも同じ）。
* **高度別の表**：表の例（0, 100, 200, 300 m）で、成分が高度について線形に補間され、最下端より下と最上端より上では端の値が一定であること、
  350° と 10° の中間（250 m）が方位 0°、風速 :math:`4\cos10^\circ` m/s になることを確かめている。
  代表風（:eq:`eq-wind-profile-mean`）は、0 m で 2 m/s、100 m で 6 m/s に直線で増える表の平均風速が 4 m/s、1 行の表がその行の風、
  互いに逆向きの風が打ち消し合う表が基準高度の風向になることを確かめている。
  基準風速・風向の差し替え（:eq:`eq-wind-scale`）では、平均風速の 2 倍、風向を 180° 回した風で、拡大率が 2 になり、最下端の風速が 4 m/s、100 m の風速が 12 m/s、
  350° の点が 180° 回った向きになり、差し替えた表の平均風速が 2 倍になる。
  基準高度で無風でも上空に風がある表は、上空の風を保ったまま拡大され（平均風速に等しい風速では拡大率 1）、地上が 0.02 m/s の表でも拡大率は 1 に近い。
  平均風速が 0.1 m/s 未満の表は、風の格子とモンテカルロ解析（風速の標準偏差が 0 でないとき）でエラーになる（単一の飛翔は拡大しないので実行できる）。
  モンテカルロ解析の拡大率は :math:`(W+\delta)/W` に一致する。
  見出しの誤り、高度が増加しない行、負の高度の行（行を示して、射点より低い行を捨てて 0 m の行を加えるよう促すエラー）、データのない表、数値でない行、``wind.profile`` の欠落、存在しないファイルはいずれもエラーになる。
* **平均風の目標**\ （``profile_mean_targets_scale_and_rotate_like_a_wind_grid_case``、``profile_mean_targets_are_checked``）：``profile_mean_speed = 15.6``、``profile_mean_direction_deg = 123`` の表は、
  平均風速と風向がその値になり、拡大率が :math:`15.6/W` で、:math:`-20` から 379 m の各高度の風が、同じ風速と風向の風の格子のケースの風とビット単位で一致する。
  風向だけを指定すると拡大率は 1 のまま、``wind.speed`` と ``wind.direction_deg`` は表を変えない。無風の表、負の風速、360° を超える風向はエラーになり、目標風速 0 の表は無風になる。
  サンプルを ``profile_mean_speed = 15.6``、``profile_mean_direction_deg = 90`` で飛ばした着地点と頂点は、15.6 m/s、4 方位の風の格子の 90° のケースと一致する（``profile_targets_fly_like_the_wind_grid_case``）。
* **地面付近の接続**：対数則を :math:`h_{\mathrm{ref}}=10` m、:math:`z_0=0.1` m、接続の高さ 0.11 m とすると、端の傾きが急（:math:`\beta=1/\ln1.1=10.5`）で、補正のない 3 次の Hermite 曲線は 0.055 m で風速が :math:`-0.142` m/s まで下がる。
  補正した曲線は、0 から :math:`h_b` までの 1001 点で風速が 0 以上かつ単調に増え、:math:`h_b` で風速がモデルの値と連続である（``ground_blend_is_non_negative_and_monotonic``）。
  べき法則の接続は、補正のない曲線と 1e-14 の相対精度で一致する（:ref:`sec-wind-blend`）。

.. _sec-verify-geo:

測地座標変換
------------

東京駅（:math:`\phi=35.681236^\circ`, :math:`\lambda=139.767125^\circ`, :math:`h=0`）を\ :eq:`eq-lla-ecef` で ECEF に変換すると

.. math::

   (X,\,Y,\,Z) = (-3\,959\,665.68,\ \ 3\,350\,073.28,\ \ 3\,699\,520.04)\ \mathrm{m}

となる。これを\ :eq:`eq-ecef-lla` で戻したときの誤差は次のとおりで、Bowring の方法 :cite:`bowring` を 1 回適用するだけで十分な精度がある。

.. list-table:: 往復変換の誤差（東京駅の上空）
   :header-rows: 1
   :widths: 25 25 25 25

   * - 楕円体高 :math:`h`
     - 緯度方向 [m]
     - 経度方向 [m]
     - 高さ [m]
   * - 0 m
     - :math:`<10^{-9}`
     - :math:`<10^{-9}`
     - :math:`<10^{-9}`
   * - 10 km
     - :math:`8\times10^{-7}`
     - :math:`<10^{-9}`
     - :math:`5\times10^{-7}`
   * - 100 km
     - :math:`7\times10^{-5}`
     - :math:`<10^{-9}`
     - :math:`5\times10^{-5}`

また、ENU で北へ 1 km 進んだ点の緯度差が、子午線曲率半径 :cite:`torge` から求めた値（北緯 34.7° で約 0.009013°）と一致することも確かめている。

運動方程式
----------

真空中の鉛直飛翔
~~~~~~~~~~~~~~~~

空気力を 0、推力を一定 :math:`F`、質量を一定 :math:`m`、射角を 90° とすると、燃焼終了（時刻 :math:`t_b`）までは等加速度運動、
その後は鉛直投げ上げになる。重力を一定 :math:`g_0` とみなせば、頂点高度は

.. math::

   a_0 = \frac{F}{m}-g_0,\qquad v_b = a_0t_b,\qquad
   z_{\mathrm{apo}} = \frac12a_0t_b^2 + \frac{v_b^2}{2g_0}

である。:math:`F=300` N、:math:`m=10` kg、:math:`t_b=2` s（:math:`a_0=20.19\ \mathrm{m/s^2}`、:math:`v_b=40.39` m/s、:math:`z_{\mathrm{apo}}\approx 123.5` m）で計算し、
IgnisYeet の結果との差が 0.2 % 未満であることを確かめている（テストの許容）。
実際の差は 0.002 %（123.551 m と 123.549 m）で、IgnisYeet が重力の高度変化（:eq:`eq-gravity` の逆 2 乗則。100 m あたり約 0.003 % の減少）を含めていることによる。重力を一定にすると、差は :math:`10^{-12}` 未満になる。
落下後の着地点が射点から動かないこと（横方向の誤差が :math:`10^{-6}` m 未満）も確かめている。

風に対する応答
~~~~~~~~~~~~~~

サンプル機体を射角 85°、北向きに打ち上げ、西風 5 m/s を与えたとき、

* 無風では東西方向のずれが 1 m 未満であること
* 弾道落下では風見効果により着地点が **風上（西）** に 20 m 以上ずれること
* パラシュート降下では着地点が **風下（東）** に 50 m 以上流されること
* 着地速度が 10 m/s 未満であり、迎角が常に有限値であること

を確かめている。これは :doc:`dispersion` で述べた物理的な振る舞いと一致する。

.. _sec-verify-ecef:

ECEF 座標での積分
~~~~~~~~~~~~~~~~~

* **座標系の幾何**：射点の高さは 0（:math:`10^{-4}` m 未満）、射点の真上 1 km の点の高さは 1 km（誤差 :math:`10^{-3}` m 未満）で、
  射点の ENU への往復で :math:`10^{-6}` m 以内に戻る。逆 2 乗則の有効重力（万有引力に遠心項を加えた正味の重力）の大きさは :math:`g_0\left(R_\oplus/(R_\oplus+h_0)\right)^2`
  （:math:`10^{-9}` m/s\ :sup:`2` 以内）で、向きは測地的な下向きである。
  北緯 34.7° の射点での遠心加速度の大きさは約 0.0277 m/s\ :sup:`2`\ （:math:`\Omega_\oplus^2R\cos\phi`）で、
  上昇する物体の Coriolis 加速度は西向きの成分を持つ。
* **平面地球との一致**：風のないサンプルで、自転なし（:math:`\Omega=0`）の ECEF モードの結果が平面地球モードに一致することを確かめている
  （重力の大きさの法則が同じなので、違いは地表の曲率だけである）。着地点の差は弾道落下で 0.10 m、パラシュート降下で 0.057 m、
  頂点高度の差は 0.5 % 未満、飛行時間の差も 0.5 % 未満である。
  自転を入れると、着地点は西へ 0.3〜10 m の範囲でずれ（テストは ``-10 < landing_east < -0.3`` を確認する。弾道落下で 1.9 m、パラシュート降下で 1.3 m）、
  北方向の位置は自転なしの結果と 20 m 以内で一致する（弾道落下で 0.004 m、パラシュート降下で 0.0005 m の差）。
* **Coriolis 力による西向きのずれ**：空力のない真空中の鉛直打上げ（推力 3000 N、質量 10 kg、燃焼 5 s、重力は :math:`g_0` 一定、北緯 35°）では、
  Coriolis 加速度の東西成分が :math:`-2\Omega_\oplus\cos\phi\,v_U` なので、着地点の東向きのずれは
  :math:`-2\Omega_\oplus\cos\phi\int z\,\mathrm{d}t` で見積もれる。この積分を理想的な鉛直飛翔の高度（頂点 111 km、飛行時間 304 s）で数値的に求めると
  :math:`-2659.3` m で、ECEF モードの結果 :math:`-2604.1` m と 2.1 % の差で一致する（テストの許容は 5 %。見積もりは 1 次近似なので、完全には一致しない）。

``rk45`` の検証
~~~~~~~~~~~~~~~

* **収束**：調和振動子 :math:`\ddot x=-x` を :math:`t=10` まで積分し、``rk4``\ （:math:`\Delta t=0.01`）の位置誤差が :math:`10^{-8}` 未満、
  ``rk45``\ （許容誤差 :math:`10^{-9}`）の位置・速度の誤差が :math:`10^{-7}` 未満であることを確かめている。
* **rk4 との一致**：3 つの飛翔について、``rk4``\ （:math:`\Delta t=1` ms）を参照として ``rk45`` の結果を比べる
  （:numref:`tbl-verify-rk45`）。頂点高度の相対差 :math:`10^{-3}` 未満、着地点の差 1 m 未満、着地時刻の差 0.05 s 未満、
  ランチャ離脱の時刻の差 2 ms 未満・速度の差 0.2 m/s 未満、右辺の評価回数が ``rk4`` より少ないこと、``rk45`` の出力時刻が単調に増加することを要求している。

.. _tbl-verify-rk45:

.. list-table:: ``rk4``\ （:math:`\Delta t=1` ms）と ``rk45`` の比較
   :header-rows: 1
   :widths: 28 18 18 18 18

   * - ケース
     - 頂点の差 [m]
     - 着地点の差 [m]
     - 評価回数 ``rk4``
     - 評価回数 ``rk45``
   * - 真空・鉛直
     - :math:`1.1\times10^{-11}`
     - :math:`<10^{-12}`
     - 44560
     - 813
   * - サンプル・弾道
     - :math:`3.4\times10^{-5}`
     - 0.0011
     - 128248
     - 2529
   * - サンプル・西風 5 m/s・パラシュート
     - :math:`2.0\times10^{-4}`
     - 0.0012
     - 791392
     - 12616

.. _sec-verify-events:

ステップ内の事象
~~~~~~~~~~~~~~~~

事象の処理（:ref:`sec-step-events`）は、真空中の解析解と、ステップに複数の事象を入れた設定で確かめている。

* **ランチャ離脱からの再開**\ （``free_flight_restarts_at_the_interpolated_rail_exit``）：射角 60°、重力一定の真空で、ランチャ上の加速度とそのあとの加速度はどちらも一定なので、
  離脱の時刻から自由飛行を始めれば、刻み 0.05 s の ``rk4`` でも厳密である。離脱の時刻と速度は解析解と :math:`10^{-9}` で、燃焼中の自由飛行の位置は :math:`10^{-7}` m で一致する。
* **離脱の速度**\ （``rail_exit_velocity_is_the_derivative_of_the_hermite_position``）：真空・鉛直で、推力が時間に線形に増える（300 N から毎秒 2000 N、10 kg）と、加速度は時間に線形、位置は三次式になり、ステップのエルミート補間は厳密である。
  刻み 0.05 s の ``rk4`` で、3 m のレールの離脱の時刻が解析解と :math:`10^{-7}` s 以内、離脱速度が :math:`10^{-5}` m/s 以内で一致する。離脱速度はエルミート補間の時間微分で、速度の線形補間ではこの精度にならない。
* **ランチャ上で後ろへ動かないこと**\ （``a_thrust_dip_on_the_rail_never_moves_the_rocket_backward``）：真空・鉛直の 3 m のレールで、300 N を 0.2 s、20 N（重さより小さい）を 1.5 s まで、そのあと 300 N を与える（10 kg）。
  ``rk4``、``rk45``、``dop853`` のどれでも、ランチャ上の記録点の高さは減らず、速さは負にならない。機体は約 1.45 m（:math:`\pm0.02` m）で止まって推力が戻るまで同じ位置に速度 0 でとどまり、1.5 s より後に離脱する。
* **着地と開傘が同じステップにあるとき**\ （``touchdown_before_deployment_in_the_same_step_wins``）：刻み 10 s で頂点と着地が 1 つのステップに入る設定で、開傘の時刻が着地より後ろなら開傘せず、
  頂点と着地の時刻が解析解と :math:`10^{-9}` s で一致する。開傘が着地より前（1 s 前）なら、開傘の時刻は頂点 + ``delay`` ちょうどで、着地はその後である。
* **sim.max_time**\ （``no_landing_is_accepted_after_max_time``）：``rk4`` と ``rk45`` で、``max_time`` を着地の 0.01 s 前にすると着地せず、出力の時刻は ``max_time`` を超えない。0.01 s 後にすれば着地する。
* **推力曲線の折れ点**\ （``fixed_steps_resolve_a_short_thrust_spike``）：0.02 s で 10 N s の短いスパイクは、刻み 0.05 s の ``rk4`` でも、折れ点で刻みを切るので取りこぼさず、頂点の差が 1 % 未満である
  （刻み 0.5 ms との比較）。燃焼終了が全推力のサンプルの推力曲線では、燃焼終了で終わるステップが燃焼中の軸力係数を使うので、``rtol = atol = 1e-11`` の ``rk45`` と ``dop853`` が完了する
  （``dop853_tight_tolerance_through_burnout``）。
* **着地の区間**\ （``touchdown_needs_a_bracket``）：高さの区間が始端で正、終端で 0 以下のときだけ着地とする。射角が 0° 以下、90° 超、非数の飛翔は ``launch.elevation_deg must be in (0, 90]`` のエラーで拒否する。
* **メインの開傘の状態**\ （``main_deployment_state_is_at_the_main_altitude``）：ドローグの開傘から 8 s 後に大きく減速する設定で、メインの高度を横切るステップの途中でも、
  記録する開傘の状態の高さは ``main_altitude`` と :math:`10^{-6}` m で一致し、報告する高さと状態の高さが一致する。

.. _sec-verify-dual:

二段開傘、頂点の補間、発散の検出
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

* **二段開傘の終端速度**：ドローグ 0.4 m\ :sup:`2`、メイン 4 m\ :sup:`2`、メインの開傘高度 300 m の設定で、``rk4`` と ``rk45`` のそれぞれについて飛ばす。
  メインの開傘の直前（高さ 340〜400 m）の対気速度はドローグの終端速度 :math:`\sqrt{2mg/(\rho\,(C_DS)_p)}` と、着地直前の対気速度と着地速度はメインの終端速度と、5 % 以内で一致する。
  記録した開傘の高さは 300 m ちょうどで、開傘の時刻は頂点の 1 s 後より遅く着地より早い。弾道落下ではメインを使わず、``main_deploy_time`` は書かれない。
* **開傘が低いとき**：メインの開傘高度が頂点より高い（:math:`10^5` m）とき、メインはドローグの開傘と同時（頂点の 1 s 後、0.01 s 以内）に開く。
* **設定の検査**：``main_cd_s`` と ``main_altitude`` の片方だけの指定、正でない値、有限でない値がエラーになる。``parachute_cd_s_scale`` は ``cd_s`` と ``main_cd_s`` の両方に掛かる。
* **エルミート補間**：高さが :math:`z(t)=100+40t-4.9t^2` の自由落下では、7 s のステップの中の頂点の位置が :math:`10^{-9}` m 以内で解析解に一致し、ステップの終端の位置は頂点より 50 m 以上低い
  （``hermite_position_is_exact_for_free_fall_and_gives_the_apogee``）。
* **発散の検出**：``sim.dt = 5`` s の ``rk4`` は、``is not physical`` を含む発散のエラーで止まる。
  風速 :math:`10^{12}` m/s を含む風の格子では、その風速の 4 ケースだけが失敗として記録され、ほかのケースは着地する。
* **設定の上限**：``sim.dt``、``rtol``、``atol``、``aero.alpha_max_deg``、``aero.mach_max``、格子点数、``aero.n_slices`` の範囲外の値と、対数則の粗度長が基準高さの半分以上の設定がエラーになる。
* **モーターの推力**：推力曲線の最初の節点が :math:`(0,F)` のとき、:math:`t=0` の推力は :math:`F` である。最初の節点が :math:`(0,0)` なら 0 から線形に立ち上がる。
  先頭に BOM のある ``.eng``、係数表、風の表を読める。

.. _sec-verify-margin:

静安定余裕、横向きの落下、開傘の延期、最大値
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

* **小迎角の余裕**\ （``axisymmetric_margin_is_the_small_angle_margin``）：テストの機体を無風と 9 m/s の風で飛ばし、軸対称表の最小静安定余裕が、各時刻（頂点まで、対気速度 10 m/s 超）の Mach 数と重心で求めた小迎角の余裕（:eq:`eq-margin-axisym`）の最小値と 0.02 cal 以内で一致する。
  ピッチとヨーの最小値は判定値に等しく、最小はランチャ離脱の時刻にあり（燃焼中に重心が前へ移るため）、``static_margin_min_t`` は離脱の時刻、``static_margin_min_mach`` は離脱時の Mach 数と 0.01 以内で一致する。
  風による離脱時の余裕の差は 0.01 cal 未満である。一方、その時刻の迎角での表示 ``stability_cal`` の最小値は、9 m/s の風で判定値より 0.3 cal 以上小さい。
* **横向きの落下の時間**\ （``broadside_time_of_the_ballistic_descent``）：``broadside_time`` が、頂点のあと迎角 60° から 120° の記録点の数 × 出力間隔と、出力間隔の 2 倍 + 0.05 s 以内で一致する。
  テストの機体は 10 s 以下で ``fell_broadside`` は偽、パラシュート降下では 0 である。重心を後ろ（圧力中心より後ろ）に置いた機体では 1 s を超える。
* **燃焼中の開傘**\ （``deployment_waits_for_burnout_when_the_motor_still_burns``）：300 N で 2 s のあと 70 N（重さより小さい）が 20 s まで続く推力で、頂点 + 1 s が燃焼中になる。
  開傘は燃焼の終わりの時刻に起こり、``deploy_delayed`` に頂点 + 1 s の時刻と推力 70 N が残る。自由飛行の推力は燃焼の終わりまで 70 N で、パラシュート降下の推力は 0、質量は乾燥質量である。
  遅れが燃焼の終わりより後になる設定と、弾道落下では、``deploy_delayed`` は書かれない。
* **着地の状態を含む最大値**\ （``maxima_include_the_touchdown_state``）：真空で落下する飛翔では最も速いのが着地の瞬間で、``rk4`` と ``rk45`` のどちらでも、``max_speed`` が着地速度に、``max_mach`` と ``max_dyn_pressure`` が着地速度の値に一致する。
* **頂点の時刻**\ （``apogee_time_is_the_zero_of_the_hermite_vertical_velocity``）：高さ :math:`z(t)=100+40t-3t^2-0.2t^3` の 3 次の運動（ステップのエルミート補間が厳密になる）の :math:`t=1` から 8 のステップで、
  頂点の時刻が鉛直速度の零点 :math:`(-6+\sqrt{132})/1.2` と :math:`10^{-12}` 以内で、その位置が :math:`10^{-9}` m 以内で一致する（鉛直速度の線形補間では 3.93 s になる）。

``dop853`` と Lie 群による姿勢積分の検証
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

* **係数と次数**：``dop853`` の定数は SciPy の係数表と :math:`10^{-15}` で一致する（76 個）。
  求積の次数条件 :math:`\sum_ib_ic_i^k=1/(k+1)`\ （:math:`k=0,\dots,7`）が :math:`10^{-12}` で成り立ち、
  :math:`y'=y\cos t` の固定刻みの積分で観測される次数は 8 である。調和振動子と Arenstorf 軌道（許容誤差 :math:`10^{-10}`）の適応積分も、
  それぞれ解析解と周期後の初期値に合う。
* **指数写像**：:math:`\exp` と :math:`\log` の往復は :math:`10^{-13}` 以内で、:math:`\mathrm{dexp}` は数値微分と、:math:`\mathrm{dexp}^{-1}` は :math:`\mathrm{dexp}` と往復で一致する
  （級数と閉じた形の切り替え :math:`s=0.1` でも連続）。
* **自由剛体**\ （:math:`I=\mathrm{diag}(1,2,3)`、:math:`\bm\omega_0=(0.3,1.0,0.4)`、:math:`t\in[0,20]`）：RKMK 法（``rk4``）の刻み :math:`h=0.01` で、
  :math:`\bigl||\bm{q}|-1\bigr|\le5.8\times10^{-15}`、エネルギーの相対誤差 :math:`7.1\times10^{-12}`、角運動量の大きさの相対誤差 :math:`4.6\times10^{-12}` である。
  観測した次数は ``rk4`` の表で 4.10、Dormand–Prince 5 次の表で 4.92 である。
  :math:`h=0.05` では、姿勢の誤差は RKMK 法が :math:`1.57\times10^{-7}` rad、RK4 と正規化が :math:`2.59\times10^{-7}` rad で、前者のほうが小さい（:numref:`fig-rigid-body-attitude`）。
* **全組合せの一致**：3 つの積分法と 2 つの姿勢の更新法の 6 通りすべてで、サンプル機体の弾道とパラシュート降下を、``rk4``\ （:math:`\Delta t=0.5` ms）を参照として比べる。
  頂点高度は 0.5 m 以内、着地点は 2 m 以内を要求し、観測した差はそれぞれ 0.001 m 以下、0.007 m 以下である。
  ECEF モードでも同じ検証をし、さらに ``dop853`` と ``lie_group`` の組合せで軌道が出力されることを確かめている。
  真空中の鉛直飛翔の頂点も、6 通りすべてで解析解と相対誤差 :math:`2\times10^{-3}` 以内で一致する。
* **ノルムの保存**：``lie_group`` では、パラシュート降下を含む飛翔全体（平面地球と ECEF、3 つの積分法）で :math:`\bigl||\bm{q}|-1\bigr|\le8.9\times10^{-15}` である（テストは :math:`10^{-12}` 未満を要求）。
* **評価回数**：滑らかなモデル（空力係数が Mach 数に依らず迎角に線形、風なし、出力間隔 0.5 s）で、``dop853`` の評価回数が ``rk45`` の 0.6 倍（許容誤差 :math:`10^{-9}`）・0.4 倍（:math:`10^{-11}`）より少なく、
  両者の頂点が 0.05 m 以内で一致する。評価回数は 7535 回に対して 4205 回、17981 回に対して 6353 回である。
* **厳しい許容誤差での着地**：``rtol = atol = 1e-10`` でべき法則の風がある弾道・パラシュート降下を、``rk45``・``dop853`` と 2 つの姿勢の更新法で計算して、
  刻み幅が下限を下回らずに着地まで完了することを確かめている。

ロール、慣性テンソル、推力の傾き、一般表
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

* **定常なロール角速度**：力の係数が 0 でロール項だけを持つ軸対称表（:math:`C_{l\delta}=2`、:math:`C_{lp}=-20`、カント 1.5°、ロールの慣性モーメント 0.001 kg m\ :sup:`2`）で鉛直に打ち上げ、
  燃焼の後半（:math:`t=1.2` から 1.9 s）のロール角速度が :math:`\omega_x=-2VC_l^{\mathrm{tot}}/(C_{lp}d)`\ （:eq:`eq-roll-steady`）と 8 % 以内で一致する
  （``axisymmetric_cant_gives_steady_roll_rate``）。
* **カントの基準**：一般表（:math:`\delta_{\mathrm{ref}}=2^\circ`、:math:`C_l=0.3`）では、カントが 2° のとき表の :math:`C_l` だけで、3° のとき :math:`C_l+C_{l\delta}\cdot1^\circ` でロールが定まる
  （差の分だけを足す。``general_table_roll_adds_only_the_cant_difference``）。軸対称表でカントが :math:`\delta_{\mathrm{ref}}` に等しいときは、表の強制項 :math:`C_{l\delta}\delta_{\mathrm{ref}}` だけが効く
  （``axisymmetric_reference_cant_with_equal_cant_keeps_table_roll``）。
* **慣性テンソルの符号**：点質量 :math:`m=2` kg を :math:`(0.3,-0.2,0)` m に置いた平行軸の項が :math:`I_{xy}=-mxy`、:math:`I_{xx}=my^2`、:math:`I_{yy}=mx^2`、:math:`I_{zz}=m(x^2+y^2)` になる
  （``point_mass_inertia_sign``。:math:`10^{-15}` 以内）。
* **一般表の静安定余裕**\ （``static_margins_separate_pitch_and_yaw``、``static_margins_of_an_axisymmetric_expansion_equal_the_axisymmetric_margin``）：ピッチの圧力中心 1.2 m、ヨーの圧力中心 0.6 m、重心 0.9 m、:math:`d=0.1` m の一般表で、余裕はピッチ :math:`+3`、ヨー :math:`-3` cal になる（:math:`10^{-9}` 以内）。軸対称表を展開した一般表では、Mach 数の格子点で軸対称表の余裕と :math:`10^{-9}` で一致する。
  ピッチで安定・ヨーで不安定な表で飛ばすと、判定はヨーの余裕で不安定になり、``sim`` は ``RESULTS NOT USABLE`` を返し、``summary.json`` の ``static_margin_yaw_min_cal`` は負、``stable`` は ``false`` になる（``general_table_unstable_in_yaw_is_not_usable``）。余裕は飛翔計算の各ステップで求めるので、軌道の点を記録しない飛翔、0.002 s ごとに記録する飛翔、1 s ごとに記録する飛翔で、``static_margin_pitch_min_cal``、``static_margin_yaw_min_cal``、``min_stability_cal``、``stability_at_rail_exit`` が一致し、
  記録した点で求めた余裕より大きくならない（ヨーの圧力中心を 1 cal 前へ動かした一般表。ヨーの余裕がピッチより 0.05 cal 以内で 1 cal 小さい。``general_table_margins_are_evaluated_at_every_step_independent_of_recording``）。
  軸対称表では、面ごとの最小値がどちらも判定値 ``min(min_stability_cal, stability_at_rail_exit)`` に等しい。
  ヨーの圧力中心を 6 cal 前へ動かすとピッチは 1 cal 超、ヨーは負の余裕で不安定と判定され、横力を 0 にした一般表ではヨーの余裕が ``null``、理由が ``yaw normal-force slope dCy/dbeta is zero`` を含み、判定できない（``general_table_yaw_instability_and_unknown_stability_are_reported``）。
  ``sim`` では、判定できない飛翔が ``RESULTS NOT USABLE: static stability could not be evaluated (at t = …`` で終わり、``stable`` が ``null``、ピッチの余裕は 1 cal を超える（``general_table_without_side_force_has_unknown_stability``）。
  判定できない飛翔は風の格子とモンテカルロ解析で別に数え、不安定な飛翔と同時にあれば両方の節を書く（``unknown_static_stability_is_not_usable``）。
* **余裕の符号**\ （``static_margins_follow_the_signed_cg_moment_derivative``）：線形の一般表（:math:`d=0.1` m、重心 0.9 m）で、法線力の傾きが物理的でない符号（:math:`\partial C_Z/\partial\alpha=+1`）でも、
  重心まわりのモーメントが角度を広げる向き（:math:`\partial C_m/\partial\alpha=12`）なら余裕は :math:`-3` cal（不安定）、物理的な符号の表ではピッチ :math:`+3`、ヨー :math:`-3` cal になる（:math:`10^{-9}` 以内）。
  物理的でない符号で復元するモーメントの面と、法線力の傾きが 0 の面は求まらず、理由（``… is positive at Mach 0.50``、``… is zero``）を返す。
* **慣性テンソルの検査**\ （``ill_conditioned_inertia_is_an_error``）：主慣性モーメントの比が :math:`10^{8}` を超える乾燥の慣性テンソル、:math:`I_{xx}=0`、負の :math:`I_{xx}` は、``inertia tensor is singular or ill-conditioned`` のエラーになり、条件のよい非対称なテンソルは飛ぶ。
* **乾燥重心の横ずれ**：乾燥重心を :math:`y_b` 方向に 0.02 m ずらすと、推進剤を含めた重心の横ずれが :math:`m_dy_d/m` になり、慣性乗積 :math:`I_{xy}` が
  :math:`-\sum m\,\Delta x\,\Delta y`\ （重心からの距離で、乾燥部分と推進剤の和）と :math:`10^{-12}` で一致する。慣性テンソルの行列式は正である
  （``off_axis_dry_cg_gives_product_of_inertia_with_propellant``）。
* **トルクなしの回転**：非対称な剛体（:math:`I_{xx}=0.02`, :math:`I_{yy}=1.5`, :math:`I_{zz}=1.8`、乗積 :math:`0.05,-0.03,0.1` kg m\ :sup:`2`、重心の横ずれあり）に
  初期角速度 :math:`(0.5,3.0,0.4)` rad/s を与え、60 s 積分する。角運動量の大きさとエネルギーの相対誤差の要求は :math:`10^{-6}` 未満で、
  3 つの積分法と 2 つの姿勢の更新法の 6 通りの組合せで、角運動量は :math:`2\times10^{-8}` 以下、エネルギーは :math:`4\times10^{-8}` 以下である
  （``torque_free_asymmetric_body_conserves_energy_and_momentum``）。
* **推力の傾き**：2° の傾きを方位 90° に与えると、加速度の :math:`z_b` 成分が :math:`T\sin\varepsilon/m`、:math:`y_b` 成分が変わらず、:math:`x_b` 成分が :math:`T(1-\cos\varepsilon)/m` だけ減る
  （:math:`10^{-9}` 以内）。ノズルは重心より後ろにあるので :math:`+y_b` まわりの角加速度が :math:`(x_{\mathrm{aft}}-x_{cg})T\sin\varepsilon/I_{yy}` になり、傾きが 0 なら 0 である
  （``misaligned_thrust_points_along_azimuth_and_pitches``）。
* **ノズルのオフセット**：:math:`y_b` 方向に 0.01 m のオフセットを与えると、:math:`z_b` まわりの角加速度が :math:`-0.01\,T/I_{zz}` になる（``nozzle_offset_gives_moment_about_the_cg``）。
* **一般表の飛翔**：サンプル機体の軸対称表と、それを展開した一般表で飛翔を比べる（無風と 5 m/s の風）。頂点高度の差は無風で 0.004 m、5 m/s の風で 0.04 m（要求は 0.5 m）である。
  一般表の最小静安定余裕は、軸対称表の飛翔の各時刻（頂点まで、対気速度 10 m/s 超）で一般表の余裕の定義（:ref:`sec-static-margin-general`）から求めた最小値と 0.02 cal 以内で一致し、迎角と横滑り角はすべて有限である（``general_table_flight_matches_axisymmetric_table``）。
  一般表の余裕は 0 から 2° の割線の傾きから求めるので、0 から 1° の割線から求める軸対称表の余裕より約 0.1 cal 小さい。
  設定ファイルのサンプルを一般表で飛ばしても、頂点高度は 4440.5 m に 0.5 m 以内で一致する（``general_table_sample_matches_axisymmetric_apogee``）。
* **ランチャ上のロール角**：仰角 80° で ``roll_deg`` を 90° にすると、:math:`+x_b` は変わらず、:math:`+y_b` が元の :math:`+z_b` に、:math:`+z_b` が元の :math:`-y_b` になる
  （``launch_roll_rotates_the_body_about_the_rail``）。
* **サンプルでのカント**：``fin_cant_deg = 1.0`` を設定すると、係数表の ``fin_cant_ref_deg`` とシミュレーションのカントが 1° になり、最大ロール角速度が 1 rad/s を超える
  （``fin_cant_rolls_the_sample_and_reaches_the_geometry``。計算では約 88 rad/s）。
* **外部の表のロール項**：ロール項がない外部の表（``method = "table"``）を読むと、:math:`C_{l\delta}>0`、:math:`C_{lp}<0` の部品積み上げ法の値が入る
  （``external_table_without_roll_terms_gets_barrowman_roll_terms``）。


.. _sec-verify-unsteady:

加速度と回転による空気力
~~~~~~~~~~~~~~~~~~~~~~~~

見かけの質量の遅れと対流の回転のモーメント、回転による法線力、Magnus 力、誘導ロール、ロール共振の確認（:ref:`sec-aero-unsteady`）を、
線形理論の解析解、細長い物体の理論の閉じた式、項を 1 つずつ入れ切りした飛翔計算の差で確かめる（``sim`` の ``flight_dyn_tests``）。どれもモデルどうしの検証で、実測との比較ではない。

* **細長い物体の理論のピッチ減衰の閉じた式との比較**\ （``pitch_damping_of_the_sample_matches_slender_body_theory``）：
  細長い物体の理論の閉じた式 :math:`C_{mq}+C_{m\dot\alpha}=-2C_{N\alpha}(l_e/d)^2`\ （:eq:`eq-sbt-damping`）と、モデルの減衰を比べる。
  これは :math:`\omega_y` の項と :math:`\dot\alpha` の項の組み立ての **内部の整合性の確認** で、独立な基準ではない。
  閉じた式の :math:`C_{N\alpha}` も、フィンの見かけの質量の大きさ（:math:`A_{f,\max}=SC_{N\alpha,f}/2`）も、同じ Barrowman の傾きから来るからである。
  モデルの値は、サンプルの表（軸力を 0 にしたもの）で 100 m/s の自由振動を計算し、Prony 法で求めた根の実部 :math:`B` から、法線力による減衰 :math:`a=C_{N\alpha}qS/(mV)` を除いて :math:`-(B-a)\,4I/(\rho VSd^2)` とする（回転による法線力は切る）。

  .. list-table:: サンプルのピッチ減衰 :math:`C_{mq}+C_{m\dot\alpha}` [1/rad]（:math:`M=0.29`）
     :header-rows: 1
     :widths: 22 26 26 26

     * - 重心
       - モデル
       - 閉じた式
       - 比
     * - 0.85 m
       - −1045.0
       - −987.7
       - 1.058
     * - 0.95 m
       - −755.7
       - −707.2
       - 1.069
     * - 0.02 m（フィンなし）
       - −888.2
       - −877.0
       - 1.013

  要求はフィン付きで 10 %、フィンなしで 3 % である。フィンなしの胴体では Barrowman の胴体の傾きが細長い物体の理論の傾きそのものなので、よく一致する。
  フィン付きの差は、フィンの局所的な迎角の和が Barrowman の圧力中心で働き、細長い物体の理論の :math:`A'` の重みの位置とずれることによる。
  見かけの質量を胴体の容積だけにすると比は 0.723 と 0.682 になり、要求から外れる。
  この確認で検出できるのは、項の符号の誤り、対流の回転の項の係数（:eq:`eq-lag-accel` の 2）、重心まわりへの移し替えの誤り、フィンの断面の見かけの質量の入れ忘れである。
  見かけの質量を Barrowman の傾きにそろえるという選び方そのものと、減衰の値の物理的な正しさ（実測との一致）は、この確認では分からない。

* **自由振動と線形理論**\ （``free_oscillation_matches_the_linear_theory``）：重力と推力のない一定の大気（:math:`C_{N\alpha}=10` /rad、圧力中心 1.0 m、重心 0.5 m、:math:`I=1` kg m\ :sup:`2`、遅れの和 :math:`L_0=10` m を 0.8 m に置いた、遅れの項を見分けやすい機体）を迎角 0.5° から放し、
  ``rk4``\ （:math:`h=0.2` ms）で積分した横流れから、Prony 法で 2 つの根を求める。
  Etkin の短周期の式 :cite:`etkin` に、回転による法線力と対流の回転の力（:math:`1-c_q-c_c`）と、見かけの質量の遅れと対流の回転のモーメントを加えた特性方程式の固有振動数と対数減衰率に、次のとおり一致する（要求はそれぞれ 1 % と 3 %）。
  この比較は実装の式を書き直した理論との一致で、上の閉じた式の比較と違い、モデルの物理の誤りは検出しない。

  .. list-table:: 自由振動の固有振動数 :math:`\omega_n` [rad/s] と対数減衰率（計算 / 線形理論）
     :header-rows: 1
     :widths: 28 36 36

     * - 入れた項
       - :math:`\omega_n`
       - 対数減衰率
     * - なし（準定常の減衰だけ）
       - 18.03310 / 18.03330
       - 2.66865 / 2.66876
     * - 遅れ
       - 17.62945 / 17.62952
       - 3.58770 / 3.58784
     * - 回転による法線力
       - 17.52694 / 17.52712
       - 2.76035 / 2.76047
     * - 遅れと回転による法線力
       - 17.11136 / 17.11140
       - 3.70283 / 3.70298

  差は振動数で 0.001 %、減衰率で 0.005 % 以下である。重心が容積の中心より前にあるこの機体では、遅れの項が減衰を 34 % 増やし、回転による法線力が振動数を 2.8 % 下げる。どちらの変化も許容より大きいので、項の入れ忘れは検出される。
* **回転する機体と Murphy の理論**\ （``spinning_body_follows_murphys_epicyclic_theory``）：同じ機体を 5 kg、:math:`I_{xx}=0.1` kg m\ :sup:`2`、ロール角速度 300 rad/s にし、``cnpa`` を :math:`+3` と :math:`-3` にして飛ばす。
  横流れの 2 つの円運動（速いモードと遅いモード）の振動数と減衰率を、Murphy のエピサイクリックな理論 :cite:`mccoy` の近似式と、特性方程式の厳密な根と比べる。

  .. list-table:: 回転する機体の 2 つのモード（計算 / 厳密な根。括弧内は Murphy の近似式）
     :header-rows: 1
     :widths: 22 39 39

     * - ``cnpa``、モード
       - 振動数 [rad/s]
       - 減衰率 [1/s]
     * - :math:`+3`、速い
       - 37.8416 / 37.8417（37.9012）
       - −2.4590 / −2.4592（−2.4574）
     * - :math:`+3`、遅い
       - −7.8955 / −7.8957（−7.9552）
       - −1.1078 / −1.1078（−1.1096）
     * - :math:`-3`、速い
       - 37.8504 / 37.8505（37.9199）
       - −1.8733 / −1.8734（−1.8732）
     * - :math:`-3`、遅い
       - −7.7963 / −7.7965（−7.8659）
       - −1.6935 / −1.6935（−1.6938）

  厳密な根とは振動数で :math:`10^{-3}`、減衰率で 1 % 以内、Murphy の近似式とは振動数で 1 %、減衰率で 5 % 以内を要求する。
  重心が圧力中心より前にあるので、``cnpa`` を負にすると Magnus のモーメントは速いモードの減衰を弱め、遅いモードの減衰を強める。計算もこの向きに変わることを確かめる。
  ただし理論は実装と同じ符号の約束で書いているので、Magnus 力の向きそのものは次の 1 ステップの確認で、McCoy の式と照らして確かめる。
* **Magnus 力の向き**\ （``magnus_force_points_along_u_b_cross_e_x``）：ロール角速度 200 rad/s、対気速度の横の成分が :math:`+z_b` 向き（迎角 2°）、``cnpa = 1`` で、Magnus 力だけの加速度の差が :math:`+y_b` 向きで大きさ :math:`qS\,C_{Np\alpha}^{*}\hat p\sin\alpha` になり（:math:`10^{-9}` 以内）、
  ほかの成分を持たない。圧力中心（1.0 m、重心 0.5 m）に働くので、:math:`z_b` まわりのモーメント :math:`(x_{cg}-x_{cp})F_y<0` を作る。
  期待する向きは、McCoy の 6 自由度の式の Magnus の加速度 :math:`(\rho Sd\,p\,C_{Np\alpha}/2m)(\bm{v}\times\bm{x})` :cite:`mccoy` に :math:`\bm{v}=(u,0,w)`、:math:`\bm{x}=\bm{e}_x` を入れた :math:`(0,w,0)` で、テストの注釈に手で導いてある。
* **対流の回転の力と Magnus のモーメント**\ （``convective_apparent_mass_force_and_magnus_moment_directions``）：軸方向の流れの中で :math:`q=2` rad/s で回る機体の対流の回転の力が :math:`-\tfrac12\rho SL_0\,qV` で :math:`-z_b` を向き（:math:`10^{-9}` 以内）、
  ``cmpa = 0.5`` の Magnus のモーメントが、ロール角速度 200 rad/s、横の対気速度 :math:`+z_b` で :math:`+z_b` を向き、大きさ :math:`qSd\,C_{mp\alpha}^{*}\hat p\sin\alpha` になる（力は変わらない）。
* **減衰の和の打ち切り**\ （``damping_sums_clamped_to_zero_give_no_strip_force``）：重心まわりの減衰の和が負で 0 に打ち切られる和では、回転による法線力も働かない。
* **回転による法線力の符号**\ （``rotation_induced_force_has_the_sign_of_the_damping_moment``）：軸方向の流れの中で :math:`\omega=(0,2,-1)` rad/s で回る機体の力の差が\ :eq:`eq-cnq-force` に :math:`10^{-12}` で一致し、:math:`-z_b` と :math:`-y_b` を向く。
  モーメントは変わらない（減衰のモーメントがすでにこの力のモーメントである）。展開した一般表でも同じ力になる。
* **風のシアの中の遅れのモーメント**\ （``lag_moment_follows_the_crossflow_acceleration_with_wind_shear``）：べき法則の風の中を 100 m/s で鉛直に上がる、回転していない機体（高さ 50 m）で、
  遅れのモーメントが\ :eq:`eq-lag-moment` と :math:`10^{-9}` で一致する。加速度は遅れの項の入り切りで変わらない（力を加えない）。
  :math:`\dot{\bm W}` を除いた式とは 20 % 以上違い、風のシアの項が効いていることを確かめる。
* **迎角による消え方**\ （``lag_fade_is_smooth_and_vanishes_beyond_30_deg``）：:eq:`eq-lag-fade` が 0° で 1、15° で :math:`\cos15^\circ`、30° 以上で 0 で、0.1° 刻みで単調に減り、1 刻みの変化が 0.02 未満である。
* **準定常の飛翔の固定**\ （``quasi_steady_switch_reproduces_the_quasi_steady_trajectory_bit_for_bit``）：遅れ、Magnus 力、回転による法線力を切った飛翔（カント 1°、重心の横ずれ 1 mm、横風 5 m/s の弾道落下）が、
  テストに固定した頂点、着地点、最大 Mach 数のビット列と **ビット単位で** 一致する（軸対称表と展開した一般表）。これは実装の回帰を見張る固定であり、独立した検査ではない。項を入れると頂点は変わり、差は 2 m 未満である。
* **誘導ロール**\ （``induced_roll_follows_the_crossflow_roll_angle_from_the_fin_plane``）：横流れがフィンの面にあるとき 0、22.5° ずれたとき :math:`C_{l4}\alpha^2`、フィンを 45° 回すと符号が反転する。展開した一般表の :math:`C_l` にこの項が入り、後ろ向きの飛行では反転する。
* **誘導ロールとフィンの枚数**\ （``induced_roll_applies_to_four_fins_only``）：付随ファイルの ``fin_count`` が 4 か記録なしなら :math:`C_{l4}` を使い、3 なら軸対称表でも展開した一般表でも 0 である。
* **ロール共振**\ （``roll_resonance_is_reported_for_a_canted_rocket_with_a_lateral_cg_offset``、``a_general_table_with_a_zero_angle_moment_is_a_trim_forcing``）：トリムの強制のない機体では記録しない。カント 2° だけ（ロールはするが非対称がない）でも記録しない。
  カント 2° と重心の横ずれ 2 mm を与えると、ランチャ離脱と頂点の間で交差を記録し（:math:`t=0.66` s、Mach 0.16、5.22 rad/s）、その時刻に最も近い記録点のロール角速度が振動数と 5 % 以内、Mach 数が 0.005 以内で一致する。``summary.json`` の往復でも値が変わらない。
  迎角 0 のピッチモーメントが 0.01 の一般表（カントなし）は、トリムの強制として交差を記録し、対称な一般表は記録しない。
* **遅れの和**\ （``lag_sums_of_an_ogive_cylinder_and_a_cone``）：接線オジーブと円柱の機体の容積が解析値と 1 % 以内（72 角形の断面による差を含む）、容積の中心が数値積分と全長の 0.5 % 以内で一致し、:math:`L_0=2\mathrm{Vol}/S` である。
  円錐では容積 :math:`\pi R^2L/3` と中心 :math:`3L/4` に :math:`10^{-12}` で一致する。フィンのない機体の和は胴体の容積の値だけである。
* **フィンの見かけの質量**\ （``fin_apparent_mass_follows_the_wing_body_section_and_the_fin_normal_force``）：フィンの和が、:eq:`eq-lag-fin-section` の断面を 10 万点で積分してフィンの法線力の傾きにそろえた値と :math:`10^{-4}` で一致し、
  その中心が付け根の翼弦の後半にあり、Mach 数による変化がフィンの法線力の傾きの比に従う。
  先端の前縁が付け根の後縁より後ろにある後退したフィン（付け根の翼弦 0.20 m、先端の前縁 0.30 m 後ろ、先端の翼弦 0.10 m）でも、
  先端の後縁までの積分と :math:`10^{-4}` で一致し、最大の半幅の区間を含む（``fin_apparent_mass_of_a_fin_swept_beyond_its_root_chord_reaches_the_full_span``）。
* **古い列の組**\ （``older_column_sets_load_with_zero_lag_and_magnus_columns``）：部品積み上げ法の表は形状の遅れの和とフィンの枚数を持ち、``cl4``、``cnpa``、``cmpa`` は 0 である。現在の形式の保存と読み込みで ``cl4``、``cnpa``、``cmpa``、``lag_l1`` が変わらない。
  14 個（``cmpa`` なし）、10 個、8 個の値の軸対称表、18 個（``cmpa`` なし）と 15 個の値の一般表は、足りない列が 0 で読める。
* **外部の表の遅れの和**\ （``external_table_without_lag_columns_gets_the_geometric_lag_sums``、``panel_and_cfd_tables_get_the_lag_sums_of_the_geometry``）：遅れと Magnus の列を持つ外部の表は警告なしでその値を使う。
  持たない表は、STL の形状から求めた遅れの和を入れて警告を 1 つ出し（実測の減衰の表なら 0 の列を書くように促す）、形状が読めなければ遅れの和を 0 のまま別の警告を出す。
  現在の形式で遅れの和を 0 と明示した表は、補わずに 0 のまま使う。``cl4`` を持ち ``fin_count`` のない表は、4 枚とみなす警告を出す。パネル法と CFD の表には、形状から求めた遅れの和が入る。
* **準定常の設定**\ （``quasi_steady_switch_flies_without_the_rate_and_acceleration_terms``）：``sim.quasi_steady = true`` のサンプルの飛翔が、加速度と回転による項のないモデルの値（頂点 4440.71 m、着地点 東 −654.73 m、北 −7708.12 m）になる。

**サンプルでの項の効果**：サンプル（``examples/sample.toml``）を、加速度と回転による項を入れて（既定）と ``sim.quasi_steady = true`` で飛ばした差は次のとおりである。

.. list-table:: サンプルの頂点と着地点（項あり / 準定常だけ）
   :header-rows: 1
   :widths: 22 26 26 26

   * - 降下
     - 頂点 [m]
     - 着地点 東 [m]
     - 着地点 北 [m]
   * - パラシュート
     - 4440.48 / 4440.71
     - −655.57 / −654.73
     - −7707.70 / −7708.12
   * - 弾道
     - 4440.48 / 4440.71
     - −1072.88 / −1071.82
     - 372.32 / 372.73

頂点は 0.24 m 下がり、着地点は 1 m ほど動く。回転による法線力だけでは頂点 −0.35 m、見かけの質量の項だけでは +0.12 m である。

モンテカルロ解析
~~~~~~~~~~~~~~~~

* **推力曲線の伸縮**：推力を 1.1 倍、時間軸を 1.5 倍にすると、燃焼時間が 1.65 s、全力積が元の 1.1 倍（110 N s）になり、
  推力曲線は :math:`F'(t)=kF(t/\tau)/\tau`\ （:eq:`eq-mc-thrust`）に従う。推進剤質量は変わらない。
* **再現性**：同じ設定を 2 回計算すると、全行の着地点が完全に一致する（8 サンプル × 2 降下モード = 16 行）。``seed`` を変えると結果が変わり、
  ``seed`` が 3 のサンプル 1 と ``seed`` が 2 のサンプル 2 は同じ値を引く。
* **標準偏差 0**：すべての標準偏差を 0 にすると、全サンプルの結果が公称の飛翔と **ビット単位で** 一致し、引いた値はすべて 0 になる。共分散は :math:`10^{-18}` 未満である。
* **風向のばらつき**：風向だけを標準偏差 20°（0.349 rad）で変えた 40 サンプルの着地点は、
  射点からの距離がほとんど変わらず（変動係数 :math:`10^{-15}`）、方位角の標準偏差が 0.353 rad で、与えた 0.349 rad とよく一致する。
  このとき :math:`1\sigma` 楕円の長半径は短半径の 3 倍を超える。
* **共分散と誤差楕円**：北から 30° の向きの長軸（±30 m）と短軸（±10 m）を持つ長方形の頂点 4 点から、
  平均、共分散（:math:`n-1=3` で割った値）、:math:`1\sigma` と :math:`3\sigma` の半径が :math:`10^{-9}` の精度で解析値に一致し、
  長軸の方位が 30° になること、東西に長い点群で 90°、南北に長い点群で 0° になることを確かめている。
* **失敗の扱い**：推力の標準偏差を 2.0 にした 30 サンプルでは、推力が機体の重量（約 103 N）より小さくなる引き（:math:`\epsilon<-0.7`）のサンプルだけが、
  「ランチャを離れなかった」として失敗する。失敗は統計から除かれ（有効サンプル数が 30 − 失敗数）、同じサンプルのパラシュート降下も同じく失敗する。
* **ばらつきの反映**：推力 +10 %、燃焼時間 +25 %、乾燥質量 +20 %、重心 +0.01 m、:math:`C_N` +10 %、:math:`C_A` −200 %（下限 0.05 倍）、
  射角 −1°、射方位 +2°、風速 −9 m/s（0 で止まる）、風向 +100°、:math:`C_DS` +50 % を同時に与えると、
  全力積 1.1 倍・燃焼時間 2.5 s、質量 12 kg・:math:`I_{yy}` 1.2 kg m\ :sup:`2`・重心 0.51 m、空力の倍率 (1.1, 0.05)、射角 89°・射方位 2°、
  風速 0・風向 10°、:math:`C_DS` 1.5 倍に反映される。
* **レールの傾き**\ （``rail_tilt_rotates_the_axis``）：鉛直のレール（射方位 0°）に面内の傾き 1° を与えると射角 89°・射方位 180°（背面側へ傾く）、-1° では 89°・0°、面に垂直な傾き 1° では 89°・射方位 90° になる。
  射角 45°、射方位 30° のレールでは、面内の 1° が射角 46°、射方位の回転 2° が射方位 32° になり、面に垂直な 0.01° は射方位を :math:`0.01^\circ/\cos45^\circ` だけ動かし、射角は 2 次の量しか変えず、公称の軸との角度は 0.01° ちょうどである（:math:`10^{-9}` 以内）。
  鉛直のレールで傾きの標準偏差だけを与えた 200 サンプルの弾道落下（無風）は、1σ 楕円の短半径が長半径の 0.7 倍を超え、全方位に同じように広がる（``rail_tilt_spread_is_isotropic_for_a_vertical_rail``）。
  射角 2°、:math:`\sigma_{\mathrm{tilt}}=3^\circ` の 20 サンプルのうち、傾けた射角が 0° 以下になるものは、``perturbed rail elevation … deg is outside (0, 90]`` の文で失敗し、残りは飛ぶ（``rail_elevation_outside_the_valid_range_fails_the_sample``）。
* **機体ごとの回転**\ （``small_rail_tilt_moves_the_body_axes_only_a_little``）：鉛直と傾いたレール（ロール角を含む 5 通り）に、:math:`10^{-3}`\ ° から 0.5° の傾きと鉛直軸まわりの回転を与えると、機体座標の 3 軸はどれも、回転角の和以上には動かない（記録した射角・射方位・ロール角の変化から作り直した飛翔の姿勢でも同じ）。鉛直のレールを面内に :math:`10^{-3}`\ ° 傾けると、射方位とロール角がともに 180° 変わる。標準偏差 :math:`10^{-3}`\ ° で引いた 50 サンプルでも、飛ばす機体の軸の動きは :math:`10^{-2}`\ ° 未満である（``drawn_rail_perturbation_turns_the_body_with_the_rail``）。
* **ロールと非対称の項**：ロールと非対称の標準偏差を 0 にした設定と、すべて有効にした設定で、最初の 11 項目の引いた値が完全に一致し、標準偏差 0 の項目は 0 を引く。
  有効にした項目は 0 でない値を引く（``old_seeds_reproduce_the_old_draws_and_new_sigmas_do_not_move_them``。先頭の乱数は ``seed`` 11 の最初の正規乱数と一致する）。
  公称の傾き 1° に :math:`z_b` 方向 1° を足すと傾きは :math:`\sqrt2` °、方位は 45° になり、公称のオフセット :math:`(0.001,0)` m に :math:`z_b` 方向 0.002 m を足すと :math:`(0.001,0.002)` m、
  カント 0.5° に 0.25° を足すと 0.75°、乾燥重心の横ずれは 0.003 m、:math:`C_l` と :math:`C_{lp}` の倍率は 1.5 と 0.5 になる。
  空力の 3 項目に :math:`0.2,-0.3,0.1` を与えると、遅れ、Magnus 力、減衰の和の倍率が 1.2、0.7、1.1 になり、回転による法線力だけの倍率は 1 のままである。
  ばらつきがないときは、推力線・カント・倍率が公称と完全に等しい（``new_perturbations_act_on_the_simulation``）。

設定ファイル
~~~~~~~~~~~~

サンプルの設定が読めること、
``gravity = "j2"`` と ``model = "flat"`` の組合せ・``profile`` で表の指定がない場合・粗度長が基準高度の半分以上の場合がエラーになること、
``aero.method = "table"`` で ``aero.table`` がないとエラーになること、外部の係数表を読んで ``sim`` が動くことを確かめている。
既定の ``aero.alpha_max_deg`` が 90 であること（``default_aero_table_reaches_90_degrees``）、
``exponent``、``roughness_length``、``ground_blend_height`` を、それを使う風モデルでだけ正の値として検査すること（``wind_profile_parameters_are_checked_only_for_the_models_that_use_them``）、
``profile_mean_speed`` と ``profile_mean_direction_deg`` が ``profile`` でだけ使えることと、``profile`` で ``wind.speed`` と ``wind.direction_deg`` を書いたときの警告（``profile_targets_and_ignored_wind_keys``）も確かめている。

``--set`` は、TOML の値（数、配列、引用符付きの文字列、真偽値）と引用符のない文字列を読み、ない表を作り、未知のキー・範囲外の値・型の誤り・``KEY=VALUE`` でない指定・空のキー・表でない項目の下のキーをエラーにする
（``set_overrides_parse_toml_values_and_bare_strings``、``set_overrides_are_checked_like_the_file``）。
``--out`` の相対パスは現在のディレクトリから解釈し（``load_with_applies_set_and_out``）、``--set`` の値は設定のハッシュに入り、ファイルに同じ値を書いた場合と同じハッシュになり、``--out`` はハッシュに入らない（``overrides_enter_the_run_hash_but_out_does_not``）。
``--aero-table`` は、キャッシュのキーが同じ表と付随ファイルをバイト単位で同じに写し、キーの違う表、読めない表、``--force`` との併用をエラーにする（``aero_table_is_reused_from_another_directory_when_the_key_matches``）。

サンプルを東風 4 m/s で弾道落下させると、迎角 90° までの表では範囲外の引き出しがなく（``angles_in_range`` が ``true``）、30° までの表では ``RESULTS NOT USABLE: … lookups beyond the aero table's angle range`` で終わり、着地距離は 90° の表の 2 倍を超える（``ballistic_descent_beyond_the_table_angles_is_not_usable``）。
飛翔が推力曲線の終わりより前に終わると ``burnout_time`` は ``null`` になり、ちょうど燃焼終了で終わる飛翔では燃焼終了の時刻になる（``burnout_time_is_reported_only_when_the_flight_reaches_it``）。
``sim`` が失敗すると、``summary.json`` が失敗の印になり、``ignisyeet.kml`` から前の飛翔の層が消える（``failed_sim_removes_the_previous_flight_from_the_kml``）。
