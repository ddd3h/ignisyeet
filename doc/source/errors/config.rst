.. _errors-config:

設定のエラーとワーニング
========================

設定ファイル、``--set``、``[resources]`` の読み込みと検査で出るメッセージ（領域 C）の一覧である。
エラーは番号 001 から 499、ワーニングは 500 から 999 である。
CFD の設定（``[aero.cfd]``）は 100 番台と 510 番台にある。

エラーは ``error[IY-Cnnn]: …``、ワーニングは ``warning[IY-Cnnn]: …`` の形で表示する。
ワーニングを ``summary.json`` などの ``warnings`` 配列に書くときは、``[IY-Cnnn] …`` の文字列のままにする。
設定の読み込みに失敗したときの終了ステータスは 1 である（``--set`` の形の誤りは、コマンドラインの使い方の誤りとして 2 である）。
検査のエラーは ``invalid config <ファイル>`` を見出しとし、原因を ``Caused by:`` の下に出す。コードは先頭の ``error[…]:`` に 1 回だけ現れる。

.. _IY-C001:

IY-C001
-------

重大度: エラー

メッセージ::

    error[IY-C001]: --set <KEY=VALUE>: invalid key "<key>" (expected a dotted path such as wind.speed)

意味
    ``--set`` のキーに空の部分がある。コマンドライン引数の検査で見つかったときは ``error: invalid value`` として終了ステータス 2 で止まる。

主な原因
    ``wind..speed=3`` や ``.speed=3`` のようにドットの前後が空。

対処
    ``wind.speed`` のようにドットでつないだ表のキーを書く。

関連
    :ref:`sec-cli-overrides`

.. _IY-C002:

IY-C002
-------

重大度: エラー

メッセージ::

    error[IY-C002]: --set <KEY=VALUE>: expected KEY=VALUE

意味
    ``--set`` の引数に ``=`` がない。

主な原因
    ``--set wind.speed 5`` のように ``=`` を忘れた。空白で区切っている。

対処
    ``--set wind.speed=5`` の形で書く。

関連
    :ref:`sec-cli-overrides`

.. _IY-C003:

IY-C003
-------

重大度: エラー

メッセージ::

    error[IY-C003]: --set <KEY=VALUE>: integer out of range (TOML integers are at most 9223372036854775807)

意味
    整数の値が TOML で扱える範囲（64 ビット符号付き整数）を超えている。

主な原因
    乱数の種などに 9223372036854775807 を超える整数を指定した。

対処
    範囲内の値にする。

関連
    :ref:`sec-cli-overrides`

.. _IY-C004:

IY-C004
-------

重大度: エラー

メッセージ::

    error[IY-C004]: --set <KEY=VALUE>: <path> is not a table

意味
    キーの途中の名前が、表ではなく値として設定ファイルにすでにある。

主な原因
    ``--set wind.speed.x=1`` のように、数値のキーの下にさらにキーを書いた。

対処
    キーのつづりと階層を設定ファイルの構成に合わせる。

関連
    :ref:`sec-cli-overrides`

.. _IY-C005:

IY-C005
-------

重大度: エラー

メッセージ::

    error[IY-C005]: invalid config <ファイル> with the --set overrides

    Caused by:
        --set <KEY=VALUE>: <TOML の読み込みエラー>

意味
    ``--set`` を適用した結果、設定を読めなくなった。この上書きを名指しして知らせる。

主な原因
    存在しないキー、型の違う値（数値のキーに文字列など）を指定した。

対処
    キーのつづりと値の型を確かめる。値が文字列のときは ``"..."`` で囲んでも囲まなくても読める。

関連
    :ref:`sec-cli-overrides`

.. _IY-C006:

IY-C006
-------

重大度: エラー

メッセージ::

    error[IY-C006]: resources.memory_gb must be 0 (use the available memory) or positive

意味
    ``[resources]`` の ``memory_gb`` が負または有限でない。

主な原因
    負の値、NaN、無限大を書いた。

対処
    0（使える分すべて）か正の値にする。

関連
    :ref:`[resources] の資源指定 <sec-cfd-resources>`

.. _IY-C007:

IY-C007
-------

重大度: エラー

メッセージ::

    error[IY-C007]: resources.nice must be between 0 and 19

意味
    ``[resources]`` の ``nice`` が 0 から 19 の外にある。

主な原因
    負の値（優先度を上げる指定）や 20 以上を書いた。

対処
    0 から 19 の整数にする。

関連
    :ref:`[resources] の資源指定 <sec-cfd-resources>`

.. _IY-C008:

IY-C008
-------

重大度: エラー

メッセージ::

    error[IY-C008]: the [environment] section was removed: move wind_speed, wind_direction_deg, wind_ref_height and wind_exponent to [wind] as speed, direction_deg, ref_height and exponent

意味
    ``[environment]`` 表は読まない。風の設定は ``[wind]`` 表にある。

主な原因
    古い設定ファイルをそのまま使った。

対処
    ``wind_speed`` などを ``[wind]`` の ``speed``、``direction_deg``、``ref_height``、``exponent`` に移す。

関連
    :ref:`sec-wind`

.. _IY-C009:

IY-C009
-------

重大度: エラー

メッセージ::

    error[IY-C009]: <key> must be a finite number, found <value>

意味
    数値の設定値が有限でない（NaN または無限大）。

主な原因
    ``nan``、``inf`` を書いた。計算の途中で生じた値を貼り付けた。

対処
    キー名を確かめ、有限の数値を書く。

関連
    :ref:`sec-config-checks`

.. _IY-C010:

IY-C010
-------

重大度: エラー

メッセージ::

    error[IY-C010]: <key> must be a finite number > 0, found <value>

意味
    正の数でなければならない設定値が 0 以下、または有限でない。

主な原因
    質量、慣性モーメント、レール長、刻み幅などに 0 や負の値を書いた。

対処
    キー名を確かめ、正の数を書く。

関連
    :ref:`sec-config-checks`

.. _IY-C011:

IY-C011
-------

重大度: エラー

メッセージ::

    error[IY-C011]: <key> must be a finite number >= 0, found <value>

意味
    0 以上の数でなければならない設定値が負、または有限でない。

主な原因
    風速、粗さ、抵抗面積などに負の値を書いた。

対処
    キー名を確かめ、0 以上の数を書く。

関連
    :ref:`sec-config-checks`

.. _IY-C012:

IY-C012
-------

重大度: エラー

メッセージ::

    error[IY-C012]: <key> must be between <lo> and <hi>, found <value>

意味
    設定値が許される範囲の外にある。

主な原因
    緯度が ±90° の外、経度が ±180° の外、風向が 0 から 360° の外など。

対処
    キー名を確かめ、範囲内の値にする。

関連
    :ref:`sec-config-checks`

.. _IY-C013:

IY-C013
-------

重大度: エラー

メッセージ::

    error[IY-C013]: rocket.cg_dry must be >= 0 (measured aft of the nose tip), found <value>

意味
    ``rocket.cg_dry`` は機首先端から後ろ向きに測るので、負の値は使えない。

主な原因
    機首先端より前を測った。符号を逆にした。

対処
    機首先端から後ろ向きに測った距離（0 以上）を書く。

関連
    :ref:`設定 [rocket] <sec-config-rocket>`

.. _IY-C014:

IY-C014
-------

重大度: エラー

メッセージ::

    error[IY-C014]: rocket.fin_override.thickness must be > 0 for aero.method = "<panel|cfd>", found <value>: the fins are meshed as biconvex sections of this thickness and fins without thickness would be left out of the mesh

意味
    パネル法と CFD の表面メッシュは、翼を両凸断面の厚みのある形で作る。厚み 0 の翼はメッシュから落ちる。

主な原因
    ``aero.method`` が ``panel`` か ``cfd``（``surface = "panel_mesh"``）で、``fin_override.thickness`` が 0 以下。

対処
    正の厚みを書く。Barrowman では 0 を書いてよい。

関連
    :ref:`設定 [rocket] <sec-config-rocket>`

.. _IY-C015:

IY-C015
-------

重大度: エラー

メッセージ::

    error[IY-C015]: launch.elevation_deg must be in (0, 90], found <value>

意味
    発射角（仰角）が 0 より大きく 90° 以下でない。

主な原因
    水平（0°）以下、または 90° より大きい値を書いた。

対処
    0 より大きく 90 以下の値にする。

関連
    :ref:`設定 [sim] <sec-config-sim>`

.. _IY-C016:

IY-C016
-------

重大度: エラー

メッセージ::

    error[IY-C016]: sim.descent = "parachute" needs recovery.enabled = true

意味
    パラシュート降下を指定したが、回収系が無効。

主な原因
    ``[recovery]`` の ``enabled`` が ``false`` のまま ``sim.descent = "parachute"`` を書いた。

対処
    ``recovery.enabled = true`` にするか、``sim.descent`` を ``ballistic`` にする。

関連
    :ref:`sec-parachute`

.. _IY-C017:

IY-C017
-------

重大度: エラー

メッセージ::

    error[IY-C017]: aero.mach_max (<value>) must be greater than aero.mach_min (<value>)

意味
    空力表のマッハ数の範囲が空。

主な原因
    ``mach_max`` が ``mach_min`` 以下。

対処
    ``mach_max`` を ``mach_min`` より大きくする。

関連
    :ref:`設定 [aero] <sec-config-aero>`

.. _IY-C018:

IY-C018
-------

重大度: エラー

メッセージ::

    error[IY-C018]: aero.mach_max = <value> is too large for a rocket flight (<= <limit>)

意味
    空力表の最大マッハ数がロケットの飛行として大きすぎる。

主な原因
    単位の取り違え、桁の誤り。

対処
    上限以下の値にする。

関連
    :ref:`設定 [aero] <sec-config-aero>`

.. _IY-C019:

IY-C019
-------

重大度: エラー

メッセージ::

    error[IY-C019]: aero.alpha_max_deg = <value> is outside (0, 180]

意味
    空力表の最大迎え角が 0 より大きく 180° 以下でない。

主な原因
    180° を超える値を書いた。

対処
    0 より大きく 180 以下の値にする。

関連
    :ref:`設定 [aero] <sec-config-aero>`

.. _IY-C020:

IY-C020
-------

重大度: エラー

メッセージ::

    error[IY-C020]: the aero table grid would have about <points> points (<内訳>; at most <limit>): increase <step_key> / aero.alpha_step_deg or reduce aero.mach_max / aero.alpha_max_deg

意味
    空力表の格子点が多すぎて、作成に時間とメモリがかかりすぎる。

主な原因
    刻みが小さすぎる。範囲が広すぎる。一般表（6 分力）は迎え角とすべりの 2 方向に点を持つので、点数が増える。

対処
    マッハ数または迎え角の刻みを大きくするか、範囲を狭める。

関連
    :ref:`設定 [aero] <sec-config-aero>`

.. _IY-C021:

IY-C021
-------

重大度: エラー

メッセージ::

    error[IY-C021]: aero.n_slices = <value> is too small: the geometry extraction needs at least <n> slices

意味
    形状抽出の断面数が少なすぎる。

主な原因
    ``aero.n_slices`` を極端に小さくした。

対処
    示された数以上にする。

関連
    :ref:`設定 [aero] <sec-config-aero>`

.. _IY-C022:

IY-C022
-------

重大度: エラー

メッセージ::

    error[IY-C022]: aero.n_slices = <value> is too large (at most <n>)

意味
    形状抽出の断面数が多すぎる。

主な原因
    桁を取り違えた。

対処
    示された数以下にする。

関連
    :ref:`設定 [aero] <sec-config-aero>`

.. _IY-C023:

IY-C023
-------

重大度: エラー

メッセージ::

    error[IY-C023]: aero.panel.transonic[1] = <value> is above the table's aero.mach_max (<value>): the transonic blend would run beyond the table; use at most <limit>

意味
    パネル法の遷音速ブレンドの上端が、空力表の最大マッハ数を超えている。

主な原因
    ``aero.panel.transonic`` の 2 番目の値が ``aero.mach_max`` より大きい。

対処
    ``transonic[1]`` を示された値以下にするか、``aero.mach_max`` を上げる。

関連
    :ref:`設定 [aero] <sec-config-aero>`

.. _IY-C024:

IY-C024
-------

重大度: エラー

メッセージ::

    error[IY-C024]: wind.roughness_length (<value>) must be smaller than half of wind.ref_height (<value>) for the log profile

意味
    対数風速分布の粗さ長が、基準高さの半分以上ある。

主な原因
    粗さ長を m ではなく mm の値で書いた。基準高さが低すぎる。

対処
    粗さ長を基準高さの半分より小さくする。

関連
    :ref:`sec-wind`

.. _IY-C025:

IY-C025
-------

重大度: エラー

メッセージ::

    error[IY-C025]: sim.dt = <value> s is too large for a rocket flight (<= <limit> s; 0.002 s is typical)

意味
    積分の刻み幅が大きすぎる。

主な原因
    単位の取り違え。

対処
    示された上限以下（標準は 0.002 s）にする。

関連
    :ref:`設定 [sim] <sec-config-sim>`

.. _IY-C026:

IY-C026
-------

重大度: エラー

メッセージ::

    error[IY-C026]: sim.dt = <value> s is too small (>= <limit> s): the run would take days; 0.002 s is typical

意味
    積分の刻み幅が小さすぎ、計算が何日もかかる。

主な原因
    桁の誤り。

対処
    示された下限以上（標準は 0.002 s）にする。

関連
    :ref:`設定 [sim] <sec-config-sim>`

.. _IY-C027:

IY-C027
-------

重大度: エラー

メッセージ::

    error[IY-C027]: sim.max_time / sim.dt = <steps> steps is too many (at most <limit>): increase sim.dt or reduce sim.max_time

意味
    積分のステップ数が多すぎる。

主な原因
    ``max_time`` が長すぎる、または ``dt`` が小さすぎる。

対処
    ``sim.dt`` を大きくするか ``sim.max_time`` を短くする。

関連
    :ref:`設定 [sim] <sec-config-sim>`

.. _IY-C028:

IY-C028
-------

重大度: エラー

メッセージ::

    error[IY-C028]: sim.output_interval = <value> s is too small (>= <limit> s): it would write an enormous trajectory and limit the adaptive step

意味
    出力間隔が小さすぎる。巨大な軌道ファイルになり、適応刻みの上限にもなる。

主な原因
    桁の誤り。

対処
    示された下限以上にする。

関連
    :ref:`設定 [sim] <sec-config-sim>`

.. _IY-C029:

IY-C029
-------

重大度: エラー

メッセージ::

    error[IY-C029]: sim.max_time / sim.output_interval = <rows> output rows is too many (at most <limit>): increase sim.output_interval

意味
    出力の行数が多すぎる。

主な原因
    ``max_time`` に対して ``output_interval`` が小さい。

対処
    ``sim.output_interval`` を大きくする。

関連
    :ref:`設定 [sim] <sec-config-sim>`

.. _IY-C030:

IY-C030
-------

重大度: エラー

メッセージ::

    error[IY-C030]: sim.output_interval = <value> s exceeds sim.max_time = <value> s

意味
    出力間隔が最大飛行時間より長い。

主な原因
    ``output_interval`` と ``max_time`` を取り違えた。

対処
    ``output_interval`` を ``max_time`` 以下にする。

関連
    :ref:`設定 [sim] <sec-config-sim>`

.. _IY-C031:

IY-C031
-------

重大度: エラー

メッセージ::

    error[IY-C031]: sim.<rtol|atol> = <value> is too loose for a rocket flight (<= <limit>; 1e-8 is typical)

意味
    適応積分器の許容誤差が緩すぎる。

主な原因
    桁の誤り。

対処
    示された上限以下（標準は 1e-8）にする。

関連
    :ref:`設定 [sim] <sec-config-sim>`

.. _IY-C032:

IY-C032
-------

重大度: エラー

メッセージ::

    error[IY-C032]: dispersion.monte_carlo.truncate_sigma must be a finite number > 0 (the bound of the clipped draws in sigma, e.g. 3.0), found <value>

意味
    モンテカルロの打ち切り幅 :math:`k` が正の有限値でない。

主な原因
    0、負の値、NaN を書いた。

対処
    3.0 のような正の数にする。打ち切らないときはキーを書かない。

関連
    :ref:`sec-mc`

.. _IY-C033:

IY-C033
-------

重大度: エラー

メッセージ::

    error[IY-C033]: rocket.fin_le must be "rounded" or "sharp"

意味
    翼の前縁形状の指定が不正。

主な原因
    つづりの誤り、ほかの語を書いた。

対処
    ``"rounded"`` か ``"sharp"`` にする。

関連
    :ref:`設定 [rocket] <sec-config-rocket>`

.. _IY-C034:

IY-C034
-------

重大度: エラー

メッセージ::

    error[IY-C034]: rocket.fin_te must be "square" or "tapered"

意味
    翼の後縁形状の指定が不正。

主な原因
    つづりの誤り、ほかの語を書いた。

対処
    ``"square"`` か ``"tapered"`` にする。

関連
    :ref:`設定 [rocket] <sec-config-rocket>`

.. _IY-C035:

IY-C035
-------

重大度: エラー

メッセージ::

    error[IY-C035]: rocket mass and inertias must be positive

意味
    乾燥質量と慣性モーメント（ixx, iyy）が正でない。

主な原因
    0 または負の値を書いた。

対処
    正の値にする。慣性モーメントは機体の質量分布から求める。

関連
    :ref:`sec-inertia-tensor`

.. _IY-C036:

IY-C036
-------

重大度: エラー

メッセージ::

    error[IY-C036]: the inertia tensor of the rocket (ixx_dry, iyy_dry, izz_dry and the products) must be positive definite

意味
    慣性テンソルが正定値でなく、物理的にありえない。

主な原因
    慣性乗積が主モーメントに比べて大きすぎる。三つの主モーメントが三角不等式を満たさない。

対処
    主モーメントと慣性乗積を測り直す、または見直す。

関連
    :ref:`sec-inertia-tensor`

.. _IY-C037:

IY-C037
-------

重大度: エラー

メッセージ::

    error[IY-C037]: rocket inertia products, CG offset, fin cant, motor misalignment and launch.roll_deg must be finite numbers

意味
    慣性乗積、横方向の重心ずれ、翼の取り付け角、推力の取り付け誤差、ロール角のいずれかが有限でない。

主な原因
    NaN または無限大を書いた。

対処
    有限の数値にする。

関連
    :ref:`設定 [rocket] <sec-config-rocket>`

.. _IY-C038:

IY-C038
-------

重大度: エラー

メッセージ::

    error[IY-C038]: aero.general_mach_step must be positive

意味
    一般表（6 分力）のマッハ刻みが正でない。

主な原因
    0 または負の値を書いた。

対処
    正の値にする。

関連
    :ref:`設定 [aero] <sec-config-aero>`

.. _IY-C039:

IY-C039
-------

重大度: エラー

メッセージ::

    error[IY-C039]: invalid aero grid

意味
    空力表の格子の指定が不正（マッハ刻み、迎え角刻み、マッハ範囲のいずれか）。

主な原因
    刻みが 0 以下、``mach_max`` が ``mach_min`` 以下。

対処
    ``aero.mach_step``、``aero.alpha_step_deg``、``aero.mach_min`` / ``mach_max`` を見直す。

関連
    :ref:`設定 [aero] <sec-config-aero>`

.. _IY-C040:

IY-C040
-------

重大度: エラー

メッセージ::

    error[IY-C040]: dispersion.wind_speeds must list at least one speed

意味
    風速格子の風速リストが空。

主な原因
    ``wind_speeds = []`` と書いた。

対処
    風速を 1 つ以上書く。

関連
    :ref:`sec-wind-grid-result`

.. _IY-C041:

IY-C041
-------

重大度: エラー

メッセージ::

    error[IY-C041]: dispersion.directions must be at least 1

意味
    風向格子の方向数が 0。

主な原因
    ``directions = 0`` と書いた。

対処
    1 以上にする。

関連
    :ref:`sec-wind-grid-result`

.. _IY-C042:

IY-C042
-------

重大度: エラー

メッセージ::

    error[IY-C042]: dispersion.directions = <value> is too large (at most <limit>)

意味
    風向の数が多すぎる。

主な原因
    桁の誤り。

対処
    示された上限以下にする。

関連
    :ref:`sec-wind-grid-result`

.. _IY-C043:

IY-C043
-------

重大度: エラー

メッセージ::

    error[IY-C043]: dispersion.wind_speeds lists <n> speeds (at most <limit>)

意味
    風速の数が多すぎる。

主な原因
    桁の誤り、刻みが細かすぎる。

対処
    示された上限以下にする。

関連
    :ref:`sec-wind-grid-result`

.. _IY-C044:

IY-C044
-------

重大度: エラー

メッセージ::

    error[IY-C044]: the wind grid would have <n> cases (wind_speeds x directions; at most <limit>)

意味
    風速格子の計算ケース数（風速数 × 方向数）が多すぎる。

主な原因
    風速と風向を細かく取りすぎた。

対処
    風速または方向の数を減らす。

関連
    :ref:`sec-wind-grid-result`

.. _IY-C045:

IY-C045
-------

重大度: エラー

メッセージ::

    error[IY-C045]: dispersion.monte_carlo.samples must be at least 1

意味
    モンテカルロのサンプル数が 0。

主な原因
    ``samples = 0`` と書いた。

対処
    1 以上にする。

関連
    :ref:`sec-mc`

.. _IY-C046:

IY-C046
-------

重大度: エラー

メッセージ::

    error[IY-C046]: dispersion.monte_carlo.samples = <value> is too large (at most <limit>)

意味
    モンテカルロのサンプル数が多すぎる。

主な原因
    桁の誤り。

対処
    示された上限以下にする。

関連
    :ref:`sec-mc`

.. _IY-C047:

IY-C047
-------

重大度: エラー

メッセージ::

    error[IY-C047]: aero.table is required when aero.method = "table"

意味
    ``aero.method = "table"`` なのに、外部空力表のパスがない。

主な原因
    ``aero.table`` を書いていない。

対処
    ``aero.table`` に表のファイルを書くか、``aero.method`` を変える。

関連
    :ref:`sec-usage-external-table`

.. _IY-C048:

IY-C048
-------

重大度: エラー

メッセージ::

    error[IY-C048]: aero.cfd.step_file = "<path>": no such file (<解決したパス>; a relative path is relative to the configuration file)

意味
    ``surface = "step"`` で指定した CAD ファイルがない。

主な原因
    パスのつづりの誤り。相対パスを設定ファイルの場所ではなく別の場所から見ている。

対処
    設定ファイルからの相対パスか絶対パスで、存在するファイルを指定する。

関連
    :ref:`sec-cfd-step`

.. _IY-C049:

IY-C049
-------

重大度: エラー

メッセージ::

    error[IY-C049]: earth.gravity = "j2" requires earth.model = "ecef"

意味
    J2 重力は ECEF モデルでだけ使える。

主な原因
    ``earth.model`` が ``flat`` のまま ``gravity = "j2"`` を書いた。

対処
    ``earth.model = "ecef"`` にするか、重力モデルを変える。

関連
    :ref:`sec-gravity`

.. _IY-C050:

IY-C050
-------

重大度: エラー

メッセージ::

    error[IY-C050]: wind.profile (CSV path) is required when wind.model = "profile"

意味
    風のモデルが ``profile`` なのに、高度別風の CSV のパスがない。

主な原因
    ``wind.profile`` を書いていない。

対処
    CSV ファイルのパスを書く。

関連
    :ref:`sec-wind-profile`

.. _IY-C051:

IY-C051
-------

重大度: エラー

メッセージ::

    error[IY-C051]: wind.ref_height must be positive

意味
    風の基準高さが正でない。

主な原因
    0 または負の値を書いた。

対処
    正の値にする。

関連
    :ref:`sec-wind`

.. _IY-C052:

IY-C052
-------

重大度: エラー

メッセージ::

    error[IY-C052]: wind.roughness_length must be positive and smaller than wind.ref_height

意味
    対数風速分布の粗さ長が正でない、または基準高さ以上。

主な原因
    粗さ長に 0 や大きすぎる値を書いた。

対処
    0 より大きく基準高さより小さい値にする。

関連
    :ref:`sec-wind`

.. _IY-C053:

IY-C053
-------

重大度: エラー

メッセージ::

    error[IY-C053]: wind.ground_blend_height must be positive and smaller than wind.ref_height

意味
    地表付近のブレンド高さが正でない、または基準高さ以上。

主な原因
    0 や基準高さ以上の値を書いた。

対処
    0 より大きく基準高さより小さい値にする。

関連
    :ref:`sec-wind-blend`

.. _IY-C054:

IY-C054
-------

重大度: エラー

メッセージ::

    error[IY-C054]: sim.dt and sim.output_interval must be positive

意味
    積分の刻み幅と出力間隔が正でない。

主な原因
    0 または負の値を書いた。

対処
    正の値にする。

関連
    :ref:`設定 [sim] <sec-config-sim>`

.. _IY-C055:

IY-C055
-------

重大度: エラー

メッセージ::

    error[IY-C055]: sim.rtol and sim.atol must be positive for the adaptive integrators (rk45, dop853)

意味
    適応積分器の相対・絶対許容誤差が正でない。

主な原因
    ``rk45`` / ``dop853`` で 0 や負の値を書いた。

対処
    正の値にする。

関連
    :ref:`sec-integrators`

.. _IY-C056:

IY-C056
-------

重大度: エラー

メッセージ::

    error[IY-C056]: <key> only applies to wind.model = "profile" (the model is "<model>"); set wind.speed / wind.direction_deg instead

意味
    ``profile_mean_speed`` と ``profile_mean_direction_deg`` は ``profile`` モデルでだけ使える。

主な原因
    別のモデルでこれらのキーを書いた。

対処
    ``wind.model = "profile"`` にするか、``wind.speed`` / ``wind.direction_deg`` を使う。

関連
    :ref:`sec-wind-scaling`

.. _IY-C057:

IY-C057
-------

重大度: エラー

メッセージ::

    error[IY-C057]: wind.profile_mean_speed must be a finite number >= 0, found <value>

意味
    ``profile_mean_speed`` が負、または有限でない。

主な原因
    負の値、NaN を書いた。

対処
    0 以上の有限の数にする。

関連
    :ref:`sec-wind-scaling`

.. _IY-C058:

IY-C058
-------

重大度: エラー

メッセージ::

    error[IY-C058]: wind.profile_mean_direction_deg must be between 0 and 360, found <value>

意味
    ``profile_mean_direction_deg`` が 0 から 360° の外にある。

主な原因
    範囲外の方位角を書いた。

対処
    0 から 360 の値にする。

関連
    :ref:`sec-wind-scaling`

.. _IY-C059:

IY-C059
-------

重大度: エラー

メッセージ::

    error[IY-C059]: rocket.nose_direction '<value>' is not one of auto, +x, -x, +y, -y, +z, -z

意味
    機首方向の指定が不正。

主な原因
    つづりの誤り、符号の書き忘れ。

対処
    ``auto``、``+x``、``-x``、``+y``、``-y``、``+z``、``-z`` のいずれかにする。

関連
    :ref:`sec-geometry-orientation`

.. _IY-C060:

IY-C060
-------

重大度: エラー

メッセージ::

    error[IY-C060]: cannot read config <path>

意味
    設定ファイルを読めない。

主な原因
    パスの誤り、読み取り権限がない。

対処
    パスと権限を確かめる。

.. _IY-C061:

IY-C061
-------

重大度: エラー

メッセージ::

    error[IY-C061]: invalid config <path>

意味
    設定ファイルが TOML として読めない。原因は続く行に出る。

主な原因
    ``=`` の欠落、引用符の閉じ忘れ、同じキーの重複。

対処
    続く行の位置の指摘に従って直す。

.. _IY-C062:

IY-C062
-------

重大度: エラー

メッセージ::

    error[IY-C062]: invalid config <path>[ with the --set overrides]

意味
    設定の内容を読み込めない。原因は続く行に出る。検査の誤り（範囲・組み合わせ）は、それぞれ別の番号で原因の行に出るので、この見出しには番号が付かない。

主な原因
    存在しないキー、型の違う値。

対処
    続く行のキー名を確かめる。

.. _IY-C063:

IY-C063
-------

重大度: エラー

メッセージ::

    error[IY-C063]: cannot read the current directory

意味
    ``--out`` の相対パスを解決するための、カレントディレクトリを取得できない。

主な原因
    カレントディレクトリが削除された、権限がない。

対処
    存在するディレクトリに移って実行するか、``--out`` に絶対パスを指定する。

.. _IY-C064:

IY-C064
-------

重大度: エラー

メッセージ::

    error[IY-C064]: rocket.stl and rocket.cad are both set; give the rocket as one STL or one CAD file
    error[IY-C064]: the rocket geometry is missing: set rocket.stl (an STL file) or rocket.cad (a STEP, IGES or BREP file)

意味
    機体の形状を ``rocket.stl`` と ``rocket.cad`` の両方に書いたか、どちらにも書いていない。どちらか一方だけを書く。

主な原因
    STL から CAD に切り替えたときに、元の ``stl`` の行を消していない。形状のキーを書き忘れた。

対処
    使う方のキーだけを残す。

関連
    :ref:`cfg-rocket-stl`、:ref:`cfg-rocket-cad`

.. _IY-C065:

IY-C065
-------

重大度: エラー

メッセージ::

    error[IY-C065]: rocket.stl_scale = <値> is set with rocket.cad: the unit of a CAD file comes from the file (rocket.cad_unit overrides it), so stl_scale does not apply
    error[IY-C065]: rocket.cad_unit is set with rocket.stl: it is the unit of rocket.cad; an STL gets its unit from rocket.stl_scale

意味
    単位のキーが形状の種類と合っていない。``stl_scale`` は STL の、``cad_unit`` は CAD ファイルのキーである。

主な原因
    STL から CAD に切り替えたときに、``stl_scale`` を残した（単位を二重に掛けないように、エラーにしている）。

対処
    CAD では ``stl_scale`` を消し、必要なら ``cad_unit`` で単位を指定する。STL では ``cad_unit`` を消す。

関連
    :ref:`cfg-rocket-stl_scale`、:ref:`cfg-rocket-cad_unit`

.. _IY-C066:

IY-C066
-------

重大度: エラー

メッセージ::

    error[IY-C066]: rocket.cad = "<path>": not a CAD file OpenCASCADE reads; use a STEP (.step, .stp), IGES (.iges, .igs) or BREP (.brep, .brp) file

意味
    ``rocket.cad`` のファイルの拡張子が CAD の形式でない。

主な原因
    STL のファイルを ``cad`` に書いた。拡張子が違う。

対処
    STL は ``rocket.stl`` に書く。CAD ファイルは拡張子を ``.step`` ``.stp`` ``.iges`` ``.igs`` ``.brep`` ``.brp`` のいずれかにする。

関連
    :ref:`cfg-rocket-cad`

.. _IY-C100:

IY-C100
-------

重大度: エラー

メッセージ::

    error[IY-C100]: aero.cfd.machs must be a non-empty list of positive Mach numbers

意味
    CFD で解くマッハ数のリストが空、または正でない値を含む。

主な原因
    ``machs = []``、0 や負の値。

対処
    正のマッハ数を 1 つ以上並べる。

関連
    :ref:`sec-cfd-flow`

.. _IY-C101:

IY-C101
-------

重大度: エラー

メッセージ::

    error[IY-C101]: aero.cfd.machs must be sorted in increasing order without duplicates

意味
    マッハ数のリストが昇順でない、または重複がある。

主な原因
    並べ替えていない。

対処
    昇順にし、重複を除く。

関連
    :ref:`sec-cfd-flow`

.. _IY-C102:

IY-C102
-------

重大度: エラー

メッセージ::

    error[IY-C102]: aero.cfd.alphas_deg must be a non-empty list of angles >= 0

意味
    迎え角のリストが空、または負の値を含む。

主な原因
    ``alphas_deg = []``、負の角。

対処
    0 以上の角を 1 つ以上並べる。

関連
    :ref:`sec-cfd-alpha-mode`

.. _IY-C103:

IY-C103
-------

重大度: エラー

メッセージ::

    error[IY-C103]: aero.cfd.alphas_deg must be sorted in increasing order without duplicates

意味
    迎え角のリストが昇順でない、または重複がある。

主な原因
    並べ替えていない。

対処
    昇順にし、重複を除く。

関連
    :ref:`sec-cfd-alpha-mode`

.. _IY-C104:

IY-C104
-------

重大度: エラー

メッセージ::

    error[IY-C104]: aero.cfd.alphas_deg must include 0 (the zero-lift axial force and the small-angle slopes)

意味
    迎え角のリストの先頭が 0 でない。0° は零揚力の軸力と小迎え角の傾きに必要。

主な原因
    0 を書き忘れた。

対処
    先頭に 0 を入れる。

関連
    :ref:`sec-cfd-alpha-mode`

.. _IY-C105:

IY-C105
-------

重大度: エラー

メッセージ::

    error[IY-C105]: aero.cfd.alphas_deg needs at least one angle above 0

意味
    迎え角が 0 だけで、傾きを求める点がない。

主な原因
    ``alphas_deg = [0.0]``。

対処
    0 より大きい角を 1 つ以上加える。

関連
    :ref:`sec-cfd-alpha-mode`

.. _IY-C106:

IY-C106
-------

重大度: エラー

メッセージ::

    error[IY-C106]: aero.cfd.<farfield|wall_size|yplus|cfl> must be positive

意味
    遠方境界、壁面格子サイズ、:math:`y^+`、CFL 数のいずれかが正でない。

主な原因
    0、負の値、NaN を書いた。

対処
    正の値にする。

関連
    :ref:`sec-cfd-domain`

.. _IY-C107:

IY-C107
-------

重大度: エラー

メッセージ::

    error[IY-C107]: aero.cfd.fin_size must be "auto" or a positive size in metres

意味
    翼面の格子サイズが ``"auto"`` でも正の長さでもない。

主な原因
    0 や負の値を書いた。

対処
    ``"auto"`` か正の長さ（m）にする。

関連
    :ref:`sec-cfd-prism`

.. _IY-C108:

IY-C108
-------

重大度: エラー

メッセージ::

    error[IY-C108]: aero.cfd.fin_size = <value> is larger than wall_size = <value>: the fin faces are never meshed coarser than the wall

意味
    翼面の格子サイズが ``wall_size`` より大きい。翼面は壁面より粗くしない。

主な原因
    ``fin_size`` を ``wall_size`` より大きく指定した。

対処
    ``wall_size`` 以下にする。

関連
    :ref:`sec-cfd-prism`

.. _IY-C109:

IY-C109
-------

重大度: エラー

メッセージ::

    error[IY-C109]: aero.cfd.fin_size = <value> is below wall_size / 50 = <value>: the mesh would be impractically large

意味
    翼面の格子サイズが ``wall_size`` の 1/50 より小さく、格子が非現実的に大きくなる。

主な原因
    桁の誤り。

対処
    ``wall_size / 50`` 以上にする。

関連
    :ref:`sec-cfd-prism`

.. _IY-C110:

IY-C110
-------

重大度: エラー

メッセージ::

    error[IY-C110]: aero.cfd.betas_deg must be a non-empty list of sideslip angles in [0, 90] deg

意味
    横滑り角のリストが空、または 0 から 90° の外の値を含む。

主な原因
    ``betas_deg = []``、範囲外の角。

対処
    0 から 90 の角を 1 つ以上並べる。

関連
    :ref:`sec-cfd-sideslip`

.. _IY-C111:

IY-C111
-------

重大度: エラー

メッセージ::

    error[IY-C111]: aero.cfd.betas_deg must be sorted in increasing order without duplicates

意味
    横滑り角のリストが昇順でない、または重複がある。

主な原因
    並べ替えていない。

対処
    昇順にし、重複を除く。

関連
    :ref:`sec-cfd-sideslip`

.. _IY-C112:

IY-C112
-------

重大度: エラー

メッセージ::

    error[IY-C112]: aero.cfd.betas_deg must start with 0

意味
    横滑り角のリストの先頭が 0 でない。

主な原因
    0 を書き忘れた。

対処
    先頭に 0 を入れる。

関連
    :ref:`sec-cfd-sideslip`

.. _IY-C113:

IY-C113
-------

重大度: エラー

メッセージ::

    error[IY-C113]: aero.cfd.roll_rate_hat must be in (0, 0.05] (p d / 2V of the roll-damping runs)

意味
    ロール減衰ケースの無次元ロール角速度が 0 より大きく 0.05 以下でない。

主な原因
    大きすぎる値は線形性が崩れる。0 は減衰を測れない。

対処
    (0, 0.05] の値にする。

関連
    :ref:`sec-cfd-roll-cases`

.. _IY-C114:

IY-C114
-------

重大度: エラー

メッセージ::

    error[IY-C114]: aero.cfd.general_mach_step must be >= 0 (0 = the [aero] mach_step)

意味
    一般表のマッハ刻みが負、または有限でない。

主な原因
    負の値を書いた。

対処
    0（``[aero]`` の ``mach_step`` を使う）か正の値にする。

関連
    :ref:`sec-cfd-general`

.. _IY-C115:

IY-C115
-------

重大度: エラー

メッセージ::

    error[IY-C115]: aero.cfd.max_geometry_error must be positive

意味
    形状誤差の許容値が正でない。

主な原因
    0 や負の値を書いた。

対処
    正の値にする。キーを書かなければ許容値は既定になる。

関連
    :ref:`sec-cfd-rebuild`

.. _IY-C116:

IY-C116
-------

重大度: エラー

メッセージ::

    error[IY-C116]: aero.cfd.surface = "stl" with model = "euler" needs base_treatment: the inviscid flow behind a flat base does not converge (…)

意味
    STL 表面でオイラー方程式を解くとき、平らな底面の後流は収束しないので、底面の扱いを選ぶ必要がある。メッセージには二つの選択肢（``fairing`` と ``flat``）の説明が続く。

主な原因
    ``base_treatment`` を書いていない。

対処
    ``base_treatment = "fairing"``\ （入力 STL を変更して円錐を付ける）か ``"flat"``\ （形を保つが収束しないことが多い）を書く。

関連
    :ref:`sec-cfd-stl-base`

.. _IY-C117:

IY-C117
-------

重大度: エラー

メッセージ::

    error[IY-C117]: aero.cfd.surface = "step" needs step_file = "<path>" (the CAD model as STEP, IGES or BREP, relative to the configuration file)

意味
    ``surface = "step"`` なのに ``step_file`` がない。

主な原因
    ``step_file`` を書いていない。

対処
    CAD ファイルのパスを書く。

関連
    :ref:`sec-cfd-step`

.. _IY-C118:

IY-C118
-------

重大度: エラー

メッセージ::

    error[IY-C118]: aero.cfd.step_file = "<path>": not a CAD file OpenCASCADE reads; use a STEP (.step, .stp), IGES (.iges, .igs) or BREP (.brep, .brp) file

意味
    ``step_file`` の拡張子が OpenCASCADE の読める形式でない。

主な原因
    STL など別の形式を指定した。

対処
    STEP、IGES、BREP のファイルにする。

関連
    :ref:`sec-cfd-step`

.. _IY-C119:

IY-C119
-------

重大度: エラー

メッセージ::

    error[IY-C119]: aero.cfd.surface = "step" with model = "euler" needs base_treatment: the inviscid flow behind a flat base does not converge (…)

意味
    CAD 表面（STEP）でオイラー方程式を解くとき、底面の扱いを選ぶ必要がある（:ref:`IY-C116` と同じ理由）。

主な原因
    ``base_treatment`` を書いていない。

対処
    ``"fairing"``（CAD 形状に円錐を融合する）か ``"flat"`` を書く。

関連
    :ref:`sec-cfd-step`

.. _IY-C120:

IY-C120
-------

重大度: エラー

メッセージ::

    error[IY-C120]: aero.cfd.step_file and aero.cfd.step_unit are only used with surface = "step"

意味
    ``step_file`` / ``step_unit`` は ``surface = "step"`` でだけ使う。

主な原因
    別の表面形式でこれらのキーを書いた。

対処
    キーを消すか、``surface = "step"`` にする。

関連
    :ref:`sec-cfd-surface`

.. _IY-C121:

IY-C121
-------

重大度: エラー

メッセージ::

    error[IY-C121]: aero.cfd.base_treatment = "fairing" needs tail_fairing > 0 (cone length in base radii)

意味
    底面にフェアリングを付ける指定だが、円錐の長さが 0。

主な原因
    ``tail_fairing = 0``。

対処
    0 より大きい値（底面半径の倍数）にする。

関連
    :ref:`sec-cfd-fairing`

.. _IY-C122:

IY-C122
-------

重大度: エラー

メッセージ::

    error[IY-C122]: aero.cfd.surface = "stl" with model = "rans": RANS needs the boundary-layer mesh, which is built only for the CAD surface (surface = "cad") and the CAD B-rep (surface = "step")

意味
    RANS の境界層格子は CAD 表面（``cad``）と STEP でだけ作る。STL 表面では使えない。

主な原因
    ``surface = "stl"`` と ``model = "rans"`` の組み合わせ。

対処
    ``surface`` を ``cad`` か ``step`` にするか、``model = "euler"`` にする。

関連
    :ref:`sec-cfd-rans-use`

.. _IY-C123:

IY-C123
-------

重大度: エラー

メッセージ::

    error[IY-C123]: aero.cfd.yplus = <value> lies in the buffer layer, where neither a resolved wall nor a wall function is valid: use yplus <= 5 (resolved wall, recommended 1) or 30..150 (standard wall function)

意味
    :math:`y^+` が緩衝層にあり、壁面解像も壁関数も成り立たない。

主な原因
    5 より大きく 30 より小さい、または 150 より大きい値を書いた。

対処
    5 以下（推奨 1）か 30 から 150 にする。

関連
    :ref:`sec-cfd-rans-use`

.. _IY-C124:

IY-C124
-------

重大度: エラー

メッセージ::

    error[IY-C124]: aero.cfd.rans_base = "cfd" needs model = "rans" (the inviscid base pressure is not used)

意味
    底面圧力を CFD から取る指定は RANS でだけ意味がある。

主な原因
    オイラー方程式で ``rans_base = "cfd"`` を書いた。

対処
    ``model = "rans"`` にするか ``rans_base`` を変える。

関連
    :ref:`sec-cfd-rans-use`

.. _IY-C125:

IY-C125
-------

重大度: エラー

メッセージ::

    error[IY-C125]: aero.cfd.roll_damping is only implemented for model = "euler"

意味
    ロール減衰ケースはオイラー方程式でだけ実装している。

主な原因
    RANS で ``roll_damping = true`` を書いた。

対処
    ``roll_damping = false`` にするか ``model = "euler"`` にする。

関連
    :ref:`sec-cfd-roll-cases`

.. _IY-C126:

IY-C126
-------

重大度: エラー

メッセージ::

    error[IY-C126]: aero.cfd.fin_roll_deg must be a finite angle

意味
    翼のロール方向の角が有限でない、または絶対値が 360° を超える。

主な原因
    NaN、大きすぎる値。

対処
    -360 から 360 の有限の角にする。

関連
    :ref:`sec-cfd-sideslip`

.. _IY-C127:

IY-C127
-------

重大度: エラー

メッセージ::

    error[IY-C127]: aero.cfd.tail_fairing must be >= 0

意味
    フェアリングの長さが負、または有限でない。

主な原因
    負の値を書いた。

対処
    0 以上の値にする。

関連
    :ref:`sec-cfd-fairing`

.. _IY-C128:

IY-C128
-------

重大度: エラー

メッセージ::

    error[IY-C128]: aero.cfd.farfield must be at least 5 body lengths

意味
    遠方境界が機体長の 5 倍より近い。

主な原因
    小さい値を書いた。

対処
    5 以上にする。

関連
    :ref:`sec-cfd-domain`

.. _IY-C129:

IY-C129
-------

重大度: エラー

メッセージ::

    error[IY-C129]: aero.cfd.convergence must be in (0, 1), e.g. 1e-6

意味
    収束判定値が (0, 1) の外にある。

主な原因
    0 以下、または 1 以上の値。

対処
    1e-6 のような値にする。

関連
    :ref:`sec-cfd-int-convergence`

.. _IY-C130:

IY-C130
-------

重大度: エラー

メッセージ::

    error[IY-C130]: aero.cfd.iterations must be at least 1

意味
    反復回数の上限が 0。

主な原因
    ``iterations = 0``。

対処
    1 以上にする。

関連
    :ref:`sec-cfd-int-convergence`

.. _IY-C131:

IY-C131
-------

重大度: エラー

メッセージ::

    error[IY-C131]: aero.cfd.multigrid must be in 0..=4 (agglomeration multigrid levels)

意味
    マルチグリッドの段数が 0 から 4 の外にある。

主な原因
    5 以上の値。

対処
    0 から 4 にする。

関連
    :ref:`[resources] の資源指定 <sec-cfd-resources>`

.. _IY-C132:

IY-C132
-------

重大度: エラー

メッセージ::

    error[IY-C132]: aero.cfd.stable_window must be at least 10 iterations

意味
    収束判定の窓幅が 10 反復未満。

主な原因
    小さな値を書いた。

対処
    10 以上にする。

関連
    :ref:`sec-cfd-int-convergence`

.. _IY-C133:

IY-C133
-------

重大度: エラー

メッセージ::

    error[IY-C133]: aero.cfd.min_iterations (<value>) must not be smaller than stable_window (<value>)

意味
    最小反復回数が収束判定の窓幅より小さい。

主な原因
    ``min_iterations`` を小さく書いた。

対処
    ``stable_window`` 以上にする。

関連
    :ref:`sec-cfd-int-convergence`

.. _IY-C134:

IY-C134
-------

重大度: エラー

メッセージ::

    error[IY-C134]: aero.cfd.timeout_minutes must be >= 0

意味
    1 ケースの制限時間が負、または有限でない。

主な原因
    負の値を書いた。

対処
    0（制限なし）か正の値にする。

関連
    :ref:`[resources] の資源指定 <sec-cfd-resources>`

.. _IY-C135:

IY-C135
-------

重大度: エラー

メッセージ::

    error[IY-C135]: aero.cfd.su2 must name the SU2_CFD executable

意味
    SU2 の実行ファイル名が空。

主な原因
    ``su2 = ""``。

対処
    ``SU2_CFD`` かそのパスを書く。

関連
    :ref:`sec-cfd-native`

.. _IY-C136:

IY-C136
-------

重大度: エラー

メッセージ::

    error[IY-C136]: fin_size must be "auto" or a size in metres, got "<text>"

意味
    ``fin_size`` に ``"auto"`` 以外の文字列を書いた。設定ファイルを読む段階のエラー。

主な原因
    つづりの誤り。

対処
    ``"auto"`` か数値（m）にする。

関連
    :ref:`sec-cfd-prism`

.. _IY-C137:

IY-C137
-------

重大度: エラー

メッセージ::

    error[IY-C137]: wall_size = <value> is not a finite length; leave it out to use the default of the model

意味
    ``wall_size`` が有限でない。設定ファイルを読む段階のエラー。

主な原因
    ``nan`` や ``inf`` を書いた。

対処
    有限の長さを書くか、キーを消してモデルの既定値を使う。

関連
    :ref:`sec-cfd-domain`

.. _IY-C500:

IY-C500
-------

重大度: ワーニング

メッセージ::

    warning[IY-C500]: --set <key> given more than once; the last value (<value>) is used

意味
    同じキーに ``--set`` を 2 回以上指定した。最後の値を使う。

主な原因
    引数の重複。

対処
    意図した値かを確かめる。不要な指定を消す。

関連
    :ref:`sec-cli-overrides`

.. _IY-C501:

IY-C501
-------

重大度: ワーニング

メッセージ::

    warning[IY-C501]: resources.threads = <n> exceeds the <m> available threads; using <m>

意味
    指定したスレッド数が使えるスレッド数を超えている。使えるスレッド数に切り下げる。

主な原因
    別のマシンの設定をそのまま使った。

対処
    ``resources.threads`` を使えるスレッド数以下にするか、0（すべて使う）にする。

関連
    :ref:`[resources] の資源指定 <sec-cfd-resources>`

.. _IY-C502:

IY-C502
-------

重大度: ワーニング

メッセージ::

    warning[IY-C502]: wind.speed is ignored for wind.model = "profile"; use wind.profile_mean_speed to scale the profile

意味
    ``profile`` モデルは ``wind.speed`` を使わない。

主な原因
    ``wind.model = "profile"`` にしたまま ``wind.speed`` が設定ファイルに残っている。

対処
    風速を変えたいときは ``wind.profile_mean_speed`` で風速分布を拡縮する。

関連
    :ref:`sec-wind-scaling`

.. _IY-C503:

IY-C503
-------

重大度: ワーニング

メッセージ::

    warning[IY-C503]: wind.direction_deg is ignored for wind.model = "profile"; use wind.profile_mean_direction_deg to rotate the profile

意味
    ``profile`` モデルは ``wind.direction_deg`` を使わない。

主な原因
    ``wind.model = "profile"`` にしたまま ``wind.direction_deg`` が設定ファイルに残っている。

対処
    風向を変えたいときは ``wind.profile_mean_direction_deg`` で風向分布を回す。

関連
    :ref:`sec-wind-scaling`

.. _IY-C510:

IY-C510
-------

重大度: ワーニング

メッセージ::

    warning[IY-C510]: aero.cfd: ranks_per_case x parallel_cases = <need> exceeds the <available> available cores; cases will be slowed down

意味
    並列に動かす SU2 のプロセス数が、使える CPU コア数を超えている。計算は遅くなる。

主な原因
    ``ranks_per_case`` と ``parallel_cases`` の積が大きい。

対処
    積を使えるコア数以下にする。

関連
    :ref:`[resources] の資源指定 <sec-cfd-resources>`

.. _IY-C511:

IY-C511
-------

重大度: ワーニング

メッセージ::

    warning[IY-C511]: aero.cfd: model = "rans" with wall_size = <value> m, finer than the RANS default <value> m: a half-model case of the sample rocket needs about 7.5 GB at 5 mm and 19 GB at 3 mm, and more on a finer mesh; check the estimate with `ignisyeet cfd-check <config> --mesh` (an explicit wall_size overrides the RANS default)

意味
    RANS で壁面格子が細かく、1 ケースのメモリが大きくなる。

主な原因
    ``wall_size`` を RANS の既定値より細かくした。

対処
    ``ignisyeet cfd-check <config> --mesh`` で見積もり、メモリに収まる ``wall_size`` にする。

関連
    :ref:`[resources] の資源指定 <sec-cfd-resources>`

.. _IY-C512:

IY-C512
-------

重大度: ワーニング

メッセージ::

    warning[IY-C512]: aero.cfd: yplus = <value> selects the standard wall function (30..150), which is not verified (the RANS verification uses a resolved wall, yplus = 1); use yplus <= 5 unless you check the results yourself

意味
    標準壁関数（:math:`y^+` が 30 から 150）は検証していない。RANS の検証は壁面解像（:math:`y^+ = 1`）で行っている。

主な原因
    ``yplus`` を 30 から 150 にした。

対処
    ``yplus`` を 5 以下にするか、結果を自分で確かめる。

関連
    :ref:`sec-cfd-rans-use`
