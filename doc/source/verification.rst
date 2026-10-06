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
     - ASCII 形式とバイナリ形式の STL が同じ三角形を与えるか
   * - ``aero::atmosphere``
     - 標準大気の代表値、温度差 0 での標準大気との一致、温度差を加えた大気の静水圧平衡、一定大気
   * - ``aero::model``、``panel``、``aero::table``
     - 部品積み上げ法、パネル法、係数表の検証（解析解、手計算、連続性、展開、保存と読み込み）。内容は :ref:`sec-vaero-barrowman`、:ref:`sec-vaero-table`、:ref:`sec-vaero-panel` にまとめる
   * - ``sim::motor``
     - 推力曲線の読み込み、力積の積分、推進剤の消費、燃焼時間・推力の伸縮（モンテカルロ用）、:math:`t=0` の推力、BOM、推力曲線の警告
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
       ロールと非対称の標準偏差が最初の 11 項目の乱数を動かさないことと、6 項目の反映。
       パーセンタイルの線形補間（``percentiles_interpolate``）、着地しないサンプルが統計を不完全にし最大着地距離を下限にすること（``non_landing_samples_make_statistics_incomplete``）、
       短い ``max_time`` で着地なしの要約になること（``short_max_time_gives_unlanded_summaries``）、6 項目の標準偏差の検査（``new_monte_carlo_sigmas_are_validated``）、
       レールの傾きの回転（``rail_tilt_rotates_the_axis``）、鉛直のレールでの等方性（``rail_tilt_spread_is_isotropic_for_a_vertical_rail``）、
       範囲外の射角のサンプルの失敗（``rail_elevation_outside_the_valid_range_fails_the_sample``）、すべての降下モードの静安定余裕（``every_dispersion_run_reports_its_stability``）
   * - ``ignisyeet``\ （``cli``）
     - 設定ファイルの読み込み（ロール・慣性テンソル・推力の傾き・``table_kind`` の項目を含む）、未知のセクションの拒否、モード間の整合性、外部係数表の読み込み、
       ロール項のない外部表への補完、
       一般表でのサンプルの頂点高度、カントによるロールとジオメトリへの反映、
       モンテカルロ CSV の列数。
       範囲外と有限でない数値の拒否（``rejects_out_of_range_and_non_finite_numbers``、``rejects_bad_motor_and_monte_carlo_values``）、既定の ``max_time`` が 3600 s であること（``default_max_time_is_one_hour``）、
       キャッシュキーが使わないブロックと CFD の資源の項目を無視し、モデルの版を含むこと（``cache_key_ignores_unused_blocks_and_cfd_resources_but_not_the_model``、``model_version_is_part_of_the_cache_key``）、
       設定のハッシュが出力先と資源に依存しないこと（``run_hash_follows_the_configuration_but_not_output_or_resources``）、
       静安定余裕・風モデルの外挿・着地しないことの警告（``stability_wind_and_landing_warnings``）、大きさの警告と機体外の重心（``implausible_size_and_cg_outside_the_body``）、
       ``summary.json`` の着地・ハッシュ・警告（``summary_json_records_landing_hash_and_warnings``）、風の格子の最大着地距離の表示（``wind_grid_line_marks_failures``）、
       サンプルの飛翔が着地して警告が一つであること（``sample_flight_lands_and_warns_about_the_extrapolated_wind``）、
       KML が古い出力と着地しなかったケースを除くこと（``stale_outputs_are_skipped``、``unlanded_wind_grid_rows_are_left_out``）、
       静的に不安定なサンプルを降下モードごとに数えて終了ステータス 3 にすること（``unstable_samples_are_counted_per_mode_and_make_the_run_unusable``）、CSV の ``stable`` 列（``dispersion_csv_files_carry_a_stable_column``）、
       実行のハッシュが外部の係数表の付属 JSON を含むこと（``run_hash_covers_the_aero_table_sidecar``）、既定のモータ位置の検査（``default_motor_position_is_checked_against_the_body``）、
       ランチャ離脱の風による迎角の警告が、風の格子の最大の風速とモンテカルロの公称の風速 + 2 :math:`\sigma` を使うこと（``rail_exit_wind_warning_uses_the_reference_speed_of_the_study``）、
       失敗した ``sim`` が古い結果の代わりに失敗の印を残すこと（``a_failed_sim_leaves_a_failure_marker_not_the_old_results``）、着地したサンプルが 1 つのときの誤差楕円の表示（``monte_carlo_panel_with_one_landed_sample_has_no_nan_ellipses``）、
       一般表の展開を数える格子の上限（``general_table_expansion_counts_for_every_method_that_expands``）、KML の説明欄の CDATA を二重にエスケープしないこと
   * - ``sim::geo``
     - 測地座標と ECEF の往復変換、ENU の北方向 1 km の緯度差
   * - ``sim::flight``
     - 真空中の鉛直飛翔と解析解の比較、横風に対する風見効果と流され、ECEF と平面地球の一致、Coriolis 力による西向きのずれ、``rk45`` と ``rk4`` の一致、積分法と姿勢の更新法の全組合せの一致、``dop853`` の評価回数、厳しい許容誤差での着地、
       ロール（定常ロール角速度、カントの基準）、慣性テンソル（符号、乾燥重心の横ずれ、トルクなしの回転）、推力の傾きとノズルのオフセット、一般表の飛翔、ランチャ上のロール角、二段開傘と頂点の補間と発散の検出（:ref:`sec-verify-dual`）、
       ステップ内の事象（ランチャ離脱からの再開、同じステップの着地と開傘、``max_time``、推力曲線の折れ点。:ref:`sec-verify-events`）

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

である。:math:`F=300` N、:math:`m=10` kg、:math:`t_b=2` s（:math:`z_{\mathrm{apo}}\approx 1.36` km）で計算し、
IgnisYeet の結果との差が 0.2 % 未満であることを確かめている。
差の大部分は、IgnisYeet が重力の高度変化（:eq:`eq-gravity`、1.4 km で約 0.04 %）を含めていることによる。
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
     - :math:`1.6\times10^{-11}`
     - :math:`<10^{-12}`
     - 44560
     - 813
   * - サンプル・弾道
     - :math:`2.1\times10^{-5}`
     - 0.0006
     - 128280
     - 2541
   * - サンプル・西風 5 m/s・パラシュート
     - 0.0004
     - 0.0025
     - 791888
     - 12640

.. _sec-verify-events:

ステップ内の事象
~~~~~~~~~~~~~~~~

事象の処理（:ref:`sec-step-events`）は、真空中の解析解と、ステップに複数の事象を入れた設定で確かめている。

* **ランチャ離脱からの再開**\ （``free_flight_restarts_at_the_interpolated_rail_exit``）：射角 60°、重力一定の真空で、ランチャ上の加速度とそのあとの加速度はどちらも一定なので、
  離脱の時刻から自由飛行を始めれば、刻み 0.05 s の ``rk4`` でも厳密である。離脱の時刻と速度は解析解と :math:`10^{-9}` で、燃焼中の自由飛行の位置は :math:`10^{-7}` m で一致する。
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
  頂点高度は 0.5 m 以内、着地点は 2 m 以内を要求し、観測した差はそれぞれ 0.001 m 以下、0.004 m 以下である。
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
  ピッチで安定・ヨーで不安定な表で飛ばすと、判定はヨーの余裕で不安定になり、``sim`` は ``RESULTS NOT USABLE`` を返し、``summary.json`` の ``static_margin_yaw_min_cal`` は負、``stable`` は ``false`` になる（``general_table_unstable_in_yaw_is_not_usable``）。落下分散の下限は、2 つの圧力中心の差 6 cal だけ余裕を下げる（``pitch_stable_yaw_unstable_flight_is_unstable``）。
* **慣性テンソルの検査**\ （``ill_conditioned_inertia_is_an_error``）：主慣性モーメントの比が :math:`10^{8}` を超える乾燥の慣性テンソル、:math:`I_{xx}=0`、負の :math:`I_{xx}` は、``inertia tensor is singular or ill-conditioned`` のエラーになり、条件のよい非対称なテンソルは飛ぶ。
* **乾燥重心の横ずれ**：乾燥重心を :math:`y_b` 方向に 0.02 m ずらすと、推進剤を含めた重心の横ずれが :math:`m_dy_d/m` になり、慣性乗積 :math:`I_{xy}` が
  :math:`-\sum m\,\Delta x\,\Delta y`（重心からの距離で、乾燥部分と推進剤の和）と :math:`10^{-12}` で一致する。慣性テンソルの行列式は正である
  （``off_axis_dry_cg_gives_product_of_inertia_with_propellant``）。
* **トルクなしの回転**：非対称な剛体（:math:`I_{xx}=0.02`, :math:`I_{yy}=1.5`, :math:`I_{zz}=1.8`、乗積 :math:`0.05,-0.03,0.1` kg m\ :sup:`2`、重心の横ずれあり）に
  初期角速度 :math:`(0.5,3.0,0.4)` rad/s を与え、60 s 積分する。角運動量の大きさとエネルギーの相対誤差の要求は :math:`10^{-6}` 未満で、
  3 つの積分法と 2 つの姿勢の更新法の 6 通りの組合せで、角運動量は :math:`2\times10^{-8}` 以下、エネルギーは :math:`4\times10^{-8}` 以下である
  （``torque_free_asymmetric_body_conserves_energy_and_momentum``）。
* **推力の傾き**：2° の傾きを方位 90° に与えると、加速度の :math:`z_b` 成分が :math:`T\sin\varepsilon/m`、:math:`y_b` 成分が変わらず、:math:`x_b` 成分が :math:`T(1-\cos\varepsilon)/m` だけ減る
  （:math:`10^{-9}` 以内）。ノズルは重心より後ろにあるので :math:`+y_b` まわりの角加速度が :math:`(x_{\mathrm{aft}}-x_{cg})T\sin\varepsilon/I_{yy}` になり、傾きが 0 なら 0 である
  （``misaligned_thrust_points_along_azimuth_and_pitches``）。
* **ノズルのオフセット**：:math:`y_b` 方向に 0.01 m のオフセットを与えると、:math:`z_b` まわりの角加速度が :math:`-0.01\,T/I_{zz}` になる（``nozzle_offset_gives_moment_about_the_cg``）。
* **一般表の飛翔**：サンプル機体の軸対称表と、それを展開した一般表で飛翔を比べる（無風と 5 m/s の風）。頂点高度の差は 0.03 m 以内（要求は 0.5 m）、
  最小静安定余裕の差は 0.1 cal 未満で、迎角と横滑り角はすべて有限である（``general_table_flight_matches_axisymmetric_table``）。
  設定ファイルのサンプルを一般表で飛ばしても、頂点高度は 4440.7 m に 0.5 m 以内で一致する（``general_table_sample_matches_axisymmetric_apogee``）。
* **ランチャ上のロール角**：仰角 80° で ``roll_deg`` を 90° にすると、:math:`+x_b` は変わらず、:math:`+y_b` が元の :math:`+z_b` に、:math:`+z_b` が元の :math:`-y_b` になる
  （``launch_roll_rotates_the_body_about_the_rail``）。
* **サンプルでのカント**：``fin_cant_deg = 1.0`` を設定すると、係数表の ``fin_cant_ref_deg`` とシミュレーションのカントが 1° になり、最大ロール角速度が 1 rad/s を超える
  （``fin_cant_rolls_the_sample_and_reaches_the_geometry``。計算では約 88 rad/s）。
* **外部の表のロール項**：ロール項がない外部の表（``method = "table"``）を読むと、:math:`C_{l\delta}>0`、:math:`C_{lp}<0` の部品積み上げ法の値が入る
  （``external_table_without_roll_terms_gets_barrowman_roll_terms``）。

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
  ばらつきがないときは、推力線・カント・倍率が公称と完全に等しい（``new_perturbations_act_on_the_simulation``）。

設定ファイル
~~~~~~~~~~~~

サンプルの設定が読めること、
``gravity = "j2"`` と ``model = "flat"`` の組合せ・``profile`` で表の指定がない場合・粗度長が基準高度の半分以上の場合がエラーになること、
``aero.method = "table"`` で ``aero.table`` がないとエラーになること、外部の係数表を読んで ``sim`` が動くことを確かめている。
