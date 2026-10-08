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
有限でない値（``nan`` や ``inf``）は、すべての数値項目で受け付けない。
各キーの型と値の範囲は、:ref:`表のページ <page-reference-config>` の各項目にある。
複数のキーにまたがる条件は次のとおりである。

.. list-table::
   :header-rows: 1
   :widths: 40 60

   * - 項目
     - 条件
   * - ``recovery.main_cd_s`` と ``recovery.main_altitude``
     - 両方を指定するか、両方とも省略する。指定するときは、どちらも有限で正
   * - ``recovery.enabled`` と ``recovery.cd_s``
     - ``recovery.enabled = true`` のときは ``recovery.cd_s`` が正（0 ではパラシュートが減速しない）
   * - ``sim.descent = "parachute"``
     - ``recovery.enabled = true`` のときだけ（:ref:`IY-C016`）。コマンドラインの ``sim --descent parachute`` も同じ
   * - ``aero.mach_min`` と ``aero.mach_max``
     - ``mach_max`` は ``mach_min`` より大きい
   * - 係数表の格子点の数（Mach 数の数 × 迎角の数）
     - 約 10\ :sup:`6` 以下。迎角の数は、0 から ``alpha_max_deg`` までの片側の格子の点の数である。
       ``table_kind = "general"`` では、外部の表（``method = "table"``）以外のすべての手法が軸対称の表を一般表へ展開する。
       軸対称の表（Mach 数 × 迎角）と一般表（Mach 数 × 迎角 × 横滑り角）のうち、点の多いほうで判定する。
       部品積み上げ法の一般表では、Mach 数の刻みに ``aero.general_mach_step`` を使う
   * - ``wind.model`` と、``wind.exponent`` ``wind.roughness_length`` ``wind.ground_blend_height``
     - そのモデルが使う項目だけを検査する（``exponent`` は ``power``、``roughness_length`` は ``log``、``ground_blend_height`` は ``power`` と ``log``）。使わないモデルでは、どの値でもエラーにしない
   * - ``wind.roughness_length``\ （``model = "log"``）
     - ``wind.ref_height`` の半分より小さい
   * - ``wind.profile_mean_speed`` / ``wind.profile_mean_direction_deg``
     - ``model = "profile"`` のときだけ指定できる（:ref:`IY-C056`）
   * - ``sim.max_time`` と ``sim.dt``
     - ``integrator = "rk4"`` では ``sim.max_time / sim.dt`` が 10\ :sup:`8` 以下。``rk45`` と ``dop853`` は ``dt`` で刻まないので、この検査はしない
   * - ``sim.max_time`` と ``sim.output_interval``
     - ``sim.output_interval`` は ``sim.max_time`` 以下。``sim.max_time / sim.output_interval`` は 10\ :sup:`7` 以下
   * - ``dispersion.wind_speeds`` と ``dispersion.directions``
     - 風の格子のケース数（``wind_speeds`` の数 × ``directions``）が 100000 以下（降下モードを数えない）
   * - ``rocket.fin_override.thickness`` と ``aero.method``
     - ``aero.method = "panel"`` と、``aero.method = "cfd"`` で ``aero.cfd.surface = "panel_mesh"`` のときは正。部品積み上げ法では、厚さはフィンの抗力にだけ使うので 0 でもよい
   * - ``[aero.panel]`` の各項目
     - 手法にかかわらず検査する（:ref:`[aero.panel] の設定項目 <sec-config-aero>`）

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

飛翔計算の前に、重心まわりの慣性テンソルを、乾燥時と燃焼中について検査する。
主慣性モーメントがすべて有限で正で、最大と最小の比が 10\ :sup:`8` 以下でなければ、
``inertia tensor is singular or ill-conditioned at t = … s (principal moments …, …, … kg m^2; the largest may be at most 1e8 times the smallest): check rocket.ixx_dry, iyy_dry, izz_dry and the products of inertia``
の形のエラーにする（:ref:`sec-inertia-tensor`）。
回収系が無効な機体をパラシュート降下で飛ばそうとすると、``the parachute descent needs recovery.enabled = true (with recovery disabled there is no parachute to deploy); fly the ballistic descent or enable [recovery]`` のエラーにする。
回収系が有効なときは、飛翔計算の前にも ``recovery.cd_s`` が有限で正であること（``recovery.cd_s must be positive and finite when recovery.enabled = true, got …``）と、``recovery.delay`` が有限で 0 以上であること（``recovery.delay must be non-negative and finite, got …``）を確かめる。

UTF-8 の BOM（バイト順マーク）が先頭にあっても読める（設定ファイル、``.eng``、外部の係数表とその JSON、風のプロファイルの表）。
