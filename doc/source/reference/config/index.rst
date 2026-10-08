.. _page-reference-config:

設定ファイル
==============

設定ファイルの形式と、各表のキーを引く。

形式と単位
----------

設定は `TOML <https://toml.io/ja/v1.0.0>`_ 形式で書く。
相対パスは設定ファイルが置かれたディレクトリを基準に解釈する。ただし ``--out`` で与える出力先は、実行時のカレントディレクトリを基準にする。
長さの単位は m、角度の単位は deg である（キーごとに別の単位のものは各項目に示す）。
機体上の位置はすべてノーズ先端から後方へ測る。
キーは ``--set 表.キー=値`` で上書きできる（:ref:`sec-cli-overrides`）。

表の一覧は次のとおりで、設定ファイルの並び（``examples/sample.toml``）と同じである。
各キーの型、既定値、値の範囲、説明は、下表のページにある。

.. list-table::
   :header-rows: 1
   :widths: 40 60

   * - 表
     - ページ
   * - ``[output]`` ``[resources]`` ``[rocket]`` ``[rocket.fin_override]`` ``[motor]`` ``[launch]`` ``[recovery]``
     - :ref:`page-reference-config-general`
   * - ``[aero]`` ``[aero.panel]``
     - :ref:`page-reference-config-aero`
   * - ``[aero.cfd]``
     - :ref:`page-reference-config-cfd`
   * - ``[earth]`` ``[atmosphere]`` ``[wind]``
     - :ref:`page-reference-config-environment`
   * - ``[sim]``
     - :ref:`page-reference-config-sim`
   * - ``[dispersion]`` ``[dispersion.monte_carlo]``
     - :ref:`page-reference-config-dispersion`

各キーの項目は ``cfg-<表>-<キー>`` の形のラベルを持つ（表の名前のドットはハイフンにする。``aero.cfd.fin_size`` は ``cfg-aero-cfd-fin_size``）。

.. toctree::
   :maxdepth: 1

   general
   aero
   cfd
   environment
   sim
   dispersion

.. _sec-config-checks:

値の検査
--------

設定ファイルは読み込み時に検査し、不正な値は項目名を示してエラー（終了ステータス 1）にする。
有限でない値（``nan`` や ``inf``）は、すべての数値項目で受け付けない。主な範囲は次のとおりである。

.. list-table::
   :header-rows: 1
   :widths: 40 60

   * - 項目
     - 条件
   * - ``launch.latitude`` / ``launch.longitude``
     - :math:`-90` から :math:`90` / :math:`-180` から :math:`180`
   * - ``launch.elevation_deg``
     - :math:`0 <` 値 :math:`\le 90`
   * - ``launch.rail_length``、``rocket.stl_scale``、``rocket.dry_mass``、``rocket.ixx_dry`` ``iyy_dry`` ``izz_dry``
     - 正
   * - ``rocket.cg_dry``
     - 0 以上（ノーズ先端から後方に測る）
   * - ``rocket.roughness``、``rocket.extra_cd``、``recovery.delay``、``wind.speed``
     - 0 以上
   * - ``recovery.cd_s``
     - 0 以上。``recovery.enabled = true`` のときは正（0 ではパラシュートが減速しない）
   * - ``recovery.main_cd_s`` と ``recovery.main_altitude``
     - 両方を指定するか、両方とも省略する。指定するときは、どちらも有限で正
   * - ``sim.descent = "parachute"``
     - ``recovery.enabled = true`` のときだけ（``error[IY-C016]: sim.descent = "parachute" needs recovery.enabled = true``）。
       コマンドラインの ``sim --descent parachute`` も同じで、回収系が無効なら ``--descent parachute needs recovery.enabled = true`` のエラーにする
   * - ``aero.mach_min`` と ``aero.mach_max``
     - ``mach_min`` は 0 以上、``mach_max`` は ``mach_min`` より大きい
   * - ``aero.mach_step``、``aero.alpha_max_deg``、``aero.alpha_step_deg``、``aero.general_mach_step``
     - 正
   * - ``aero.mach_max``
     - 10 以下（ロケットの飛翔の範囲を外れる）
   * - ``aero.alpha_max_deg``
     - 180 以下
   * - 係数表の格子点の数（Mach 数の数 × 迎角の数）
     - 約 10\ :sup:`6` 以下。迎角の数は、0 から ``alpha_max_deg`` までの片側の格子の点の数である（メッセージは ``the aero table grid would have about … points (… Mach x … alpha; at most 1e6)`` の形）。
       部品積み上げ法の一般表（``method = "barrowman"`` かつ ``table_kind = "general"``）では、Mach 数の刻みに ``aero.general_mach_step`` を使い、メッセージも ``aero.general_mach_step`` を挙げる。
       ``table_kind = "general"`` では、外部の表（``method = "table"``）以外のすべての手法（部品積み上げ法、パネル法、CFD）が軸対称の表を一般表へ展開するので、
       軸対称の表（Mach 数 × 迎角）と一般表（Mach 数 × 迎角 × 横滑り角）のうち、点の多いほうで判定する（メッセージは ``the aero table grid would have about … points``）
   * - ``aero.n_slices``
     - 20 以上 10000 以下（形状抽出に必要な断面の数）
   * - ``wind.ref_height``
     - 正
   * - ``wind.exponent``、``wind.roughness_length``、``wind.ground_blend_height``
     - 正。そのモデルが使う項目だけを検査する（``exponent`` は ``power``、``roughness_length`` は ``log``、``ground_blend_height`` は ``power`` と ``log``）。使わないモデルでは、どの値でもエラーにしない
   * - ``wind.profile_mean_speed`` / ``wind.profile_mean_direction_deg``
     - ``model = "profile"`` のときだけ指定できる（ほかのモデルでは ``error[IY-C056]: wind.profile_mean_speed only applies to wind.model = "profile" (the model is "power"); set wind.speed / wind.direction_deg instead`` の形のエラー）。
       ``profile_mean_speed`` は有限で 0 以上（``error[IY-C057]: wind.profile_mean_speed must be a finite number >= 0, found …``）、``profile_mean_direction_deg`` は 0 以上 360 以下（``error[IY-C058]: wind.profile_mean_direction_deg must be between 0 and 360, found …``）
   * - ``wind.roughness_length``\ （``model = "log"``）
     - ``wind.ref_height`` の半分より小さい
   * - ``sim.dt``、``sim.max_time``、``sim.output_interval``、``sim.rtol``、``sim.atol``
     - 正
   * - ``sim.dt``
     - 10\ :sup:`-5` s 以上 0.05 s 以下（0.002 s が標準）。小さすぎると ``error[IY-C026]: sim.dt = … s is too small (>= 1e-5 s): the run would take days; 0.002 s is typical`` のエラー。
       ``integrator = "rk4"`` では、さらに ``sim.max_time / sim.dt`` が 10\ :sup:`8` 以下（超えると ``sim.max_time / sim.dt = 3.600e8 steps is too many (at most 1e8): increase sim.dt or reduce sim.max_time``）。``rk45`` と ``dop853`` は ``dt`` で刻まないので、この 2 つの検査はしない
   * - ``sim.rtol``、``sim.atol``
     - 10\ :sup:`-3` 以下（10\ :sup:`-8` が標準）
   * - ``sim.output_interval``
     - ``sim.max_time`` 以下、かつ 10\ :sup:`-4` s 以上。``sim.max_time / sim.output_interval`` は 10\ :sup:`7` 以下（出力の行数の上限。超えるときは ``sim.output_interval`` を大きくする）
   * - ``wind.direction_deg``
     - 0 以上 360 以下（``wind.direction_deg must be between 0 and 360, found 400`` の形のエラー）。``model = "profile"`` の表の風向の列も、各行が 0 以上 360 以下
   * - ``dispersion.wind_speeds`` の各要素、``[dispersion.monte_carlo]`` の各標準偏差
     - 0 以上
   * - ``dispersion.wind_speeds``
     - 1 個以上 100 個以下の要素（空のリストはエラー）
   * - ``dispersion.directions``
     - 1 以上 360 以下
   * - 風の格子のケース数（``wind_speeds`` の数 × ``directions``）
     - 100000 以下（降下モードを数えない）
   * - ``dispersion.monte_carlo.samples``
     - 1 以上 10\ :sup:`6` 以下（0 は ``error[IY-C045]: dispersion.monte_carlo.samples must be at least 1``）
   * - ``dispersion.monte_carlo.truncate_sigma``
     - 指定するときは有限で正
   * - ``rocket.fin_override.thickness``
     - ``aero.method = "panel"`` と、``aero.method = "cfd"`` で ``aero.cfd.surface = "panel_mesh"``\ （フィンを両凸の断面としてメッシュにする）のときは正（``rocket.fin_override.thickness must be > 0 for aero.method = "panel", found 0``。厚さのないフィンはメッシュから抜け落ちる）。部品積み上げ法では、厚さはフィンの抗力にだけ使うので 0 でもよい
   * - ``[aero.panel]`` の各項目
     - 手法にかかわらず検査する（:ref:`[aero.panel] の設定項目 <sec-config-aero>`）
   * - ``atmosphere.density`` ``sound_speed`` ``viscosity``\ （``model = "constant"``）
     - 有限で正
   * - ``atmosphere.temperature_offset``\ （``model = "us1976"``）
     - 有限で、−186.946 K より大きい（モデルの範囲で最も低い標準温度 186.946 K に足して、絶対温度が正になる値）

どのセクションも、未知のキーがあるとキーの名前を示してエラーにする（``rol_deg`` のような綴りの誤りが黙って無視されないようにするためである）。

入力ファイルが読めないときは、エラーにファイルのパスを示す（STL、モータの ``.eng``、風のプロファイル、外部の係数表とその付随ファイル）。
STL は、頂点の座標に有限でない値（``NaN`` や ``inf``）があると、三角形の番号を示してエラーにする。
STL の表面が閉じていない（開いた辺が辺の 1 % を超えるか、機体の基準直径の 0.1 倍より幅の広い穴がある）ときもエラーにする（:ref:`sec-geometry-warnings`）。
バイナリ STL は、見出しに書かれた三角形の数に必要なバイト数よりファイルが短いと、``binary STL declares 3 triangles (needs 234 bytes) but the file has 191 bytes: the file is truncated or corrupt`` の形（数は例）で、宣言した数、必要なバイト数、実際のバイト数を示してエラーにする。
``.eng`` の検査は :ref:`sec-propulsion-eng` に、外部の係数表の検査は :ref:`sec-aero-table-format` にある。

形状を抽出したあとで、次の場合にエラーにする。

* ``rocket.stl_scale`` を掛けた機体の全長が 0.1 m から 20 m の範囲の外にある。メッセージは ``body length … m is outside the plausible 0.1-20 m range: check the STL units and rocket.stl_scale`` の形で、``stl_scale`` の現在の値を添える。
* 乾燥重心 ``rocket.cg_dry`` が機体の全長より後方にある。
* モータ（``.eng`` の長さ）が機体の全長より長い。メッセージは ``the motor (… m) is longer than the body (… m): check motor.eng (the motor length in its header, in millimetres) and rocket.stl_scale`` （1 mm の許容がある）。
* モータが機体の外にある。モータ後端の位置 ``motor.aft_x`` がモータの長さ（``.eng`` の長さ）より前、または機体の全長より後方にある（1 mm の許容がある）。
  メッセージは ``motor.aft_x = 0.2 m puts the motor outside the body: the motor is … m long and the body … m, so motor.aft_x (the motor's aft end, measured aft of the nose tip) must be between … and … m`` の形である。
  省略したときは先頭が ``motor.aft_x (default: the body length … m) puts the motor outside the body`` になる。

全長と乾燥重心の検査のエラーでは、形状の抽出の警告（:ref:`sec-geometry-warnings`）を、メッセージの後に 1 行ずつ ``geometry warning: …`` の形で続ける。

積分が発散して状態に有限でない値が現れたときも、エラーにする（``error[IY-S081]: integration diverged at t = … s; reduce sim.dt or tighten rtol/atol, or check the aero table and mass properties``。:ref:`sec-integrators`）。値が有限でも、速さが 8 km/s を超えるか高さが 1000 km を超えたときは、発散とみなして同じ形のエラーにする。
モンテカルロ解析では、そのサンプルを失敗として数える。風の格子では、そのケースだけを失敗として記録し、ほかのケースの計算は続ける（:ref:`sec-wind-grid-failed`）。

飛翔計算の前に、重心まわりの慣性テンソルを、乾燥時と、燃焼中（燃焼時間を 32 等分した時刻と推力曲線の点の時刻）について検査する。
主慣性モーメントがすべて有限で正で、最大と最小の比が 10\ :sup:`8` 以下でなければ、
``inertia tensor is singular or ill-conditioned at t = … s (principal moments …, …, … kg m^2; the largest may be at most 1e8 times the smallest): check rocket.ixx_dry, iyy_dry, izz_dry and the products of inertia``
の形のエラーにする（:ref:`sec-inertia-tensor`）。
回収系が無効な機体をパラシュート降下で飛ばそうとすると、``the parachute descent needs recovery.enabled = true (with recovery disabled there is no parachute to deploy); fly the ballistic descent or enable [recovery]`` のエラーにする。
回収系が有効なときは、飛翔計算の前にも ``recovery.cd_s`` が有限で正であること（``recovery.cd_s must be positive and finite when recovery.enabled = true, got …``）と、``recovery.delay`` が有限で 0 以上であること（``recovery.delay must be non-negative and finite, got …``）を確かめる。

UTF-8 の BOM（バイト順マーク）が先頭にあっても読める（設定ファイル、``.eng``、外部の係数表とその JSON、風のプロファイルの表）。
