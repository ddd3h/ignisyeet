.. _errors-sim:

飛行計算のエラーとワーニング
============================

飛行計算と分散計算（``sim``）、モーターファイルと風の読み込みが出力するエラーとワーニングの一覧である。エラーは ``error[IY-Snnn]: …``、ワーニングは ``warning[IY-Snnn]: …`` の形で表示する。山括弧 ``<…>`` は実行時の値に置き換わる箇所である。

.. _IY-S001:

IY-S001
-------

重大度: エラー

メッセージ::

    error[IY-S001]: empty .eng file

意味
    .eng ファイルが空である。

主な原因
    ファイルが空、または別のファイルを指している。

対処
    ``motor.eng`` のパスと内容を確認する。

.. _IY-S002:

IY-S002
-------

重大度: エラー

メッセージ::

    error[IY-S002]: line <hn>: bad number '<s>' in .eng header (<what>)

意味
    .eng の見出し行に数値でない項目がある。

主な原因
    項目の数が足りない、単位付きで書いた、全角文字が混ざっている。

対処
    見出し行を RASP 形式（名称、直径 mm、長さ mm、遅延、推進剤質量 kg、全質量 kg、製造者）に直す。

関連
    :ref:`page-theory-propulsion`

.. _IY-S003:

IY-S003
-------

重大度: エラー

メッセージ::

    error[IY-S003]: line <hn>: malformed .eng header: expected 7 fields

意味
    .eng の見出し行の項目が 7 個ではない。

主な原因
    見出しが欠けている、改行で分かれている。

対処
    見出し行を 7 項目にする。

関連
    :ref:`page-theory-propulsion`

.. _IY-S004:

IY-S004
-------

重大度: エラー

メッセージ::

    error[IY-S004]: line <hn>: <what> must be finite, got '<s>'

意味
    .eng の見出し行の数値が有限でない。

主な原因
    ``nan`` や ``inf`` が書かれている。

対処
    有限の数値に直す。

.. _IY-S005:

IY-S005
-------

重大度: エラー

メッセージ::

    error[IY-S005]: line <hn>: diameter and length must be positive

意味
    .eng の直径または長さが正でない。

主な原因
    見出しの単位が mm でなく 0 になっている。

対処
    直径と長さを mm で正の値にする。

.. _IY-S006:

IY-S006
-------

重大度: エラー

メッセージ::

    error[IY-S006]: line <hn>: propellant mass must be non-negative, got <propellant>

意味
    .eng の推進剤質量が負である。

主な原因
    符号の誤り、単位の誤り。

対処
    推進剤質量を 0 以上の kg で書く。

.. _IY-S007:

IY-S007
-------

重大度: エラー

メッセージ::

    error[IY-S007]: line <hn>: total mass <total> kg is smaller than the propellant mass <propellant> kg

意味
    .eng の全質量が推進剤質量より小さい。

主な原因
    全質量と推進剤質量の列を取り違えた。

対処
    全質量（ケース込み）が推進剤質量以上になるよう直す。

.. _IY-S008:

IY-S008
-------

重大度: エラー

メッセージ::

    error[IY-S008]: line <n>: multiple motors in this file (second header '<l>'): keep one motor per file

意味
    1 つのファイルに複数のモーターがある。

主な原因
    複数モーターの配布ファイルをそのまま指定した。

対処
    使うモーターだけを残した 1 モーター 1 ファイルにする。

.. _IY-S009:

IY-S009
-------

重大度: エラー

メッセージ::

    error[IY-S009]: line <n>: bad thrust line '<l>'

意味
    推力曲線の行を数値として読めない。

主な原因
    列が足りない、数値でない文字がある。

対処
    その行を「時刻 s 推力 N」の 2 列にする。

関連
    :ref:`page-theory-propulsion`

.. _IY-S010:

IY-S010
-------

重大度: エラー

メッセージ::

    error[IY-S010]: line <n>: time and thrust must be finite, got '<l>'

意味
    推力曲線の時刻または推力が有限でない。

主な原因
    ``nan`` や ``inf`` が書かれている。

対処
    有限の数値に直す。

.. _IY-S011:

IY-S011
-------

重大度: エラー

メッセージ::

    error[IY-S011]: line <n>: time must be non-negative, got <t>

意味
    推力曲線の時刻が負である。

主な原因
    符号の誤り。

対処
    時刻を 0 以上にする。

.. _IY-S012:

IY-S012
-------

重大度: エラー

メッセージ::

    error[IY-S012]: line <n>: thrust must be non-negative, got <f>

意味
    推力曲線の推力が負である。

主な原因
    符号の誤り、測定のオフセット。

対処
    推力を 0 以上にする。

.. _IY-S013:

IY-S013
-------

重大度: エラー

メッセージ::

    error[IY-S013]: line <n>: thrust curve times must strictly increase, got <t> after <value>

意味
    推力曲線の時刻が単調に増えていない。

主な原因
    行の並べ替え、同じ時刻の重複。

対処
    時刻が厳密に増える順に並べる。

.. _IY-S014:

IY-S014
-------

重大度: エラー

メッセージ::

    error[IY-S014]: thrust curve has no points

意味
    推力曲線に点がない。

主な原因
    見出しの後に推力の行がない。

対処
    推力の行を加える。

.. _IY-S015:

IY-S015
-------

重大度: エラー

メッセージ::

    error[IY-S015]: thrust curve must have a positive finite total impulse

意味
    推力曲線の全力積が正の有限値でない。

主な原因
    推力がすべて 0 である。

対処
    正の推力を含む曲線にする。

.. _IY-S020:

IY-S020
-------

重大度: エラー

メッセージ::

    error[IY-S020]: adaptive integrator step size fell below <MIN_STEP> s at t = <t> s; loosen sim.rtol/sim.atol or use rk4

意味
    適応刻みの積分器が刻みの下限を下回った。

主な原因
    状態が急変している、許容誤差が厳しすぎる、空力表や質量特性が不連続である。

対処
    ``sim.rtol`` と ``sim.atol`` を緩める、``sim.integrator = "rk4"`` を使う、空力表と質量特性を確認する。

関連
    :ref:`page-theory-dynamics`

.. _IY-S030:

IY-S030
-------

重大度: エラー

メッセージ::

    error[IY-S030]: wind profile is empty

意味
    風プロファイルの CSV が空である。

主な原因
    ファイルが空、または別のファイルを指している。

対処
    ``wind.profile`` のパスと内容を確認する。

関連
    :ref:`page-theory-wind`

.. _IY-S031:

IY-S031
-------

重大度: エラー

メッセージ::

    error[IY-S031]: wind profile header must be "altitude_m,speed,direction_deg", found "<header>"

意味
    風プロファイルの見出し行が決められた形でない。

主な原因
    列名の誤り、列の順序の違い。

対処
    見出しを ``altitude_m,speed,direction_deg`` にする。

関連
    :ref:`sec-wind-profile`

.. _IY-S032:

IY-S032
-------

重大度: エラー

メッセージ::

    error[IY-S032]: bad wind profile row "<l>"

意味
    風プロファイルの行を 3 個の有限な数として読めない。

主な原因
    列の不足、数値でない文字、``nan``。

対処
    その行を「高度 m, 風速, 風向 度」の 3 列にする。

関連
    :ref:`sec-wind-profile`

.. _IY-S033:

IY-S033
-------

重大度: エラー

メッセージ::

    error[IY-S033]: wind profile speed must not be negative (row "<l>")

意味
    風プロファイルの風速が負である。

主な原因
    符号の誤り。

対処
    風速を 0 以上にする。

.. _IY-S034:

IY-S034
-------

重大度: エラー

メッセージ::

    error[IY-S034]: wind profile direction must be between 0 and 360 degrees (row "<l>")

意味
    風プロファイルの風向が 0 から 360 度の範囲にない。

主な原因
    単位の誤り、負の角度。

対処
    風向を 0 以上 360 以下にする。

.. _IY-S035:

IY-S035
-------

重大度: エラー

メッセージ::

    error[IY-S035]: wind profile altitude must be ≥ 0 m above the launch site (row "<l>"): drop levels below the launch site and add a 0 m row

意味
    風プロファイルの高度が射点より低い。

主な原因
    海抜高度で書いた。

対処
    射点からの高度で書き、射点より低い行を除いて 0 m の行を加える。

関連
    :ref:`sec-wind-profile`

.. _IY-S036:

IY-S036
-------

重大度: エラー

メッセージ::

    error[IY-S036]: wind profile altitudes must be strictly ascending (row "<l>")

意味
    風プロファイルの高度が厳密に増えていない。

主な原因
    行の並べ替え、同じ高度の重複。

対処
    高度が増える順に並べる。

.. _IY-S037:

IY-S037
-------

重大度: エラー

メッセージ::

    error[IY-S037]: wind profile has no data rows

意味
    風プロファイルにデータ行がない。

主な原因
    見出し行だけのファイル。

対処
    データ行を加える。

.. _IY-S038:

IY-S038
-------

重大度: エラー

メッセージ::

    error[IY-S038]: wind.ref_height must be positive

意味
    ``wind.ref_height`` が正でない。

主な原因
    0 や負の値を書いた。

対処
    正の高さ（m）にする。

関連
    :ref:`sec-wind`

.. _IY-S039:

IY-S039
-------

重大度: エラー

メッセージ::

    error[IY-S039]: wind.speed must be a finite number >= 0, found <value>

意味
    ``wind.speed`` が 0 以上の有限値でない。

主な原因
    負の値、``nan``、``inf``。

対処
    0 以上の有限値にする。

.. _IY-S040:

IY-S040
-------

重大度: エラー

メッセージ::

    error[IY-S040]: wind.direction_deg must be a finite number, found <value>

意味
    ``wind.direction_deg`` が有限値でない。

主な原因
    ``nan`` や ``inf``。

対処
    有限の角度にする。

.. _IY-S041:

IY-S041
-------

重大度: エラー

メッセージ::

    error[IY-S041]: wind.ground_blend_height must be positive and smaller than wind.ref_height

意味
    ``wind.ground_blend_height`` が正でない、または ``wind.ref_height`` 以上である。

主な原因
    値の大小の取り違え。

対処
    ``0 < wind.ground_blend_height < wind.ref_height`` にする。

関連
    :ref:`sec-wind-blend`

.. _IY-S042:

IY-S042
-------

重大度: エラー

メッセージ::

    error[IY-S042]: wind.exponent must be positive

意味
    ``wind.exponent`` が正でない。

主な原因
    0 や負の値。

対処
    正の指数にする（1/7 が標準）。

.. _IY-S043:

IY-S043
-------

重大度: エラー

メッセージ::

    error[IY-S043]: wind.roughness_length must be positive

意味
    ``wind.roughness_length`` が正でない。

主な原因
    0 や負の値。

対処
    正の粗度長にする。

.. _IY-S044:

IY-S044
-------

重大度: エラー

メッセージ::

    error[IY-S044]: wind.roughness_length (<value>) must be smaller than half of wind.ref_height (<value>) for the log profile

意味
    ``wind.roughness_length`` が対数則に使える大きさでない。

主な原因
    粗度長が基準高さの半分以上である。

対処
    粗度長を基準高さの半分より小さくする。

.. _IY-S045:

IY-S045
-------

重大度: エラー

メッセージ::

    error[IY-S045]: wind.profile (CSV path) is required when wind.model = "profile"

意味
    ``wind.model = "profile"`` なのに ``wind.profile`` がない。

主な原因
    CSV のパスを書き忘れた。

対処
    ``wind.profile`` に CSV のパスを書く。

.. _IY-S046:

IY-S046
-------

重大度: エラー

メッセージ::

    error[IY-S046]: cannot read wind profile <value>

意味
    風プロファイルの CSV を読めない。

主な原因
    パスの誤り、読み取り権限がない。

対処
    パスと権限を確認する。

.. _IY-S048:

IY-S048
-------

重大度: エラー

メッセージ::

    error[IY-S048]: wind.profile_mean_speed must be a finite number >= 0, found <target>

意味
    ``wind.profile_mean_speed`` が 0 以上の有限値でない。

主な原因
    負の値、``nan``。

対処
    0 以上の有限値にする。

関連
    :ref:`sec-wind-scaling`

.. _IY-S049:

IY-S049
-------

重大度: エラー

メッセージ::

    error[IY-S049]: wind.profile_mean_direction_deg must be between 0 and 360 degrees, found <dir>

意味
    ``wind.profile_mean_direction_deg`` が 0 から 360 度の範囲にない。

主な原因
    単位の誤り。

対処
    0 以上 360 以下にする。

.. _IY-S050:

IY-S050
-------

重大度: エラー

メッセージ::

    error[IY-S050]: wind profile is calm (mean speed <speed> m/s): it cannot be scaled to wind.profile_mean_speed / rotated to wind.profile_mean_direction_deg

意味
    風プロファイルがほぼ無風で、平均風速や平均風向への換算ができない。

主な原因
    プロファイルの風速がすべて 0 に近い。

対処
    風のあるプロファイルを使うか、``wind.profile_mean_speed`` と ``wind.profile_mean_direction_deg`` を外す。

関連
    :ref:`sec-wind-scaling`

.. _IY-S051:

IY-S051
-------

重大度: エラー

メッセージ::

    error[IY-S051]: wind profile is calm (mean speed <value> m/s): it cannot be scaled for dispersion; use wind.model = "constant"/"power" for a wind grid

意味
    風プロファイルがほぼ無風で、風速グリッドに換算できない。

主な原因
    無風のプロファイルで ``dispersion.mode = "wind_grid"`` を使った。

対処
    ``wind.model = "constant"`` か ``"power"`` を使う。

関連
    :ref:`page-theory-dispersion`

.. _IY-S052:

IY-S052
-------

重大度: エラー

メッセージ::

    error[IY-S052]: wind profile is calm (mean speed <value> m/s): the Monte Carlo wind perturbations cannot be applied to it; set dispersion.monte_carlo.wind_speed and wind_direction_deg to 0, or use a profile with wind

意味
    風プロファイルがほぼ無風で、モンテカルロの風の摂動を適用できない。

主な原因
    無風のプロファイルでモンテカルロを使った。

対処
    ``dispersion.monte_carlo.wind_speed`` と ``wind_direction_deg`` を 0 にするか、風のあるプロファイルを使う。

関連
    :ref:`page-theory-dispersion`

.. _IY-S053:

IY-S053
-------

重大度: エラー

メッセージ::

    error[IY-S053]: wind speed and direction must be finite numbers, found <speed> m/s from <direction_deg> deg

意味
    風速と風向が有限の数でない。

主な原因
    ``nan`` や ``inf`` の風を指定した。

対処
    有限の風速と風向にする。

.. _IY-S054:

IY-S054
-------

重大度: エラー

メッセージ::

    error[IY-S054]: gravity model "j2" needs a spherical/ellipsoidal Earth: set earth.model = "ecef" (or use another gravity model with earth.model = "flat")

意味
    重力モデル ``j2`` を平面地球で使っている。

主な原因
    ``earth.model = "flat"`` のまま ``j2`` を選んだ。

対処
    ``earth.model = "ecef"`` にするか、別の重力モデルを選ぶ。

関連
    :ref:`sec-gravity`

.. _IY-S060:

IY-S060
-------

重大度: エラー

メッセージ::

    error[IY-S060]: dispersion mode "monte_carlo" returns per-sample data: call run_monte_carlo

意味
    ``dispersion.mode = "monte_carlo"`` の計算が、風の格子の計算として実行された。

主な原因
    ソフトウェアの誤り。

対処
    ``dispersion.mode`` を確認する。通常は表示されない。

.. _IY-S061:

IY-S061
-------

重大度: エラー

メッセージ::

    error[IY-S061]: dispersion.monte_carlo.samples must be at least 1

意味
    ``dispersion.monte_carlo.samples`` が 1 未満である。

主な原因
    0 を書いた。

対処
    1 以上にする。

関連
    :ref:`sec-mc`

.. _IY-S062:

IY-S062
-------

重大度: エラー

メッセージ::

    error[IY-S062]: dispersion.monte_carlo.<name> must be a finite number >= 0 (1-sigma), found <v>

意味
    ``dispersion.monte_carlo`` の標準偏差が 0 以上の有限値でない。

主な原因
    負の値、``nan``。

対処
    0 以上の有限値（1 シグマ）にする。

関連
    :ref:`tbl-mc-perturb`

.. _IY-S063:

IY-S063
-------

重大度: エラー

メッセージ::

    error[IY-S063]: dispersion.monte_carlo.truncate_sigma must be a finite number > 0 (the bound of the clipped draws in sigma, e.g. 3.0), found <k>

意味
    ``dispersion.monte_carlo.truncate_sigma`` が正の有限値でない。

主な原因
    0 や負の値。

対処
    正の値（例 3.0）にする。

.. _IY-S064:

IY-S064
-------

重大度: エラー

メッセージ::

    error[IY-S064]: perturbed rail elevation <el> deg is outside (0, 90]

意味
    摂動を加えたレール仰角が (0, 90] 度の範囲外になった。

主な原因
    ``launch.elevation_deg`` が小さい、または大きいのに ``elevation`` のシグマが大きい。

対処
    仰角のシグマを小さくするか、``truncate_sigma`` で摂動を打ち切る。そのサンプルは失敗として数える。

関連
    :ref:`sec-mc-rail`

.. _IY-S070:

IY-S070
-------

重大度: エラー

メッセージ::

    error[IY-S070]: recovery.cd_s must be positive and finite when recovery.enabled = true, got <value>

意味
    ``recovery.cd_s`` が正の有限値でない。

主な原因
    ``recovery.enabled = true`` で 0 や負の値を書いた。

対処
    正の ``CdS`` にする。

.. _IY-S071:

IY-S071
-------

重大度: エラー

メッセージ::

    error[IY-S071]: recovery.delay must be non-negative and finite, got <value>

意味
    ``recovery.delay`` が 0 以上の有限値でない。

主な原因
    負の値。

対処
    0 以上の秒数にする。

.. _IY-S072:

IY-S072
-------

重大度: エラー

メッセージ::

    error[IY-S072]: recovery.main_cd_s must be positive and finite, got <c>

意味
    ``recovery.main_cd_s`` が正の有限値でない。

主な原因
    0 や負の値。

対処
    正の値にする。

.. _IY-S073:

IY-S073
-------

重大度: エラー

メッセージ::

    error[IY-S073]: recovery.main_altitude must be positive and finite, got <a>

意味
    ``recovery.main_altitude`` が正の有限値でない。

主な原因
    0 や負の値。

対処
    正の高度にする。

.. _IY-S074:

IY-S074
-------

重大度: エラー

メッセージ::

    error[IY-S074]: recovery.main_cd_s and recovery.main_altitude must be given together (dual deploy) or both omitted

意味
    ``recovery.main_cd_s`` と ``recovery.main_altitude`` の片方だけが指定されている。

主な原因
    二段展開の設定が揃っていない。

対処
    両方を書くか、両方を外す。

.. _IY-S075:

IY-S075
-------

重大度: エラー

メッセージ::

    error[IY-S075]: inertia tensor is singular or ill-conditioned at t = <t> s (principal moments <value>, <value>, <value> kg m^2; the largest may be at most <MAX_INERTIA_CONDITION> times the smallest): check rocket.ixx_dry, iyy_dry, izz_dry and the products of inertia

意味
    慣性テンソルが特異、または条件数が大きすぎる。

主な原因
    ``rocket.ixx_dry`` 等の値が 0 に近い、慣性乗積が大きすぎる。

対処
    慣性モーメントと慣性乗積を確認する。

関連
    :ref:`慣性モーメントの推定 <sec-usage-inertia-estimate>`

.. _IY-S076:

IY-S076
-------

重大度: エラー

メッセージ::

    error[IY-S076]: sim.dt must be positive

意味
    ``sim.dt`` が正でない。

主な原因
    0 や負の値。

対処
    正の刻みにする。

.. _IY-S077:

IY-S077
-------

重大度: エラー

メッセージ::

    error[IY-S077]: launch.elevation_deg must be in (0, 90], found <elev>

意味
    ``launch.elevation_deg`` が (0, 90] 度の範囲にない。

主な原因
    0 度や 90 度超を書いた。

対処
    範囲内の仰角にする。

.. _IY-S078:

IY-S078
-------

重大度: エラー

メッセージ::

    error[IY-S078]: the parachute descent needs recovery.enabled = true (with recovery disabled there is no parachute to deploy); fly the ballistic descent or enable [recovery]

意味
    パラシュート降下を選んだが回収装置が無効である。

主な原因
    ``recovery.enabled = false`` のままパラシュート降下を要求した。

対処
    ``[recovery]`` を有効にするか、弾道降下を使う。

.. _IY-S079:

IY-S079
-------

重大度: エラー

メッセージ::

    error[IY-S079]: sim.rtol, sim.atol and sim.output_interval must be positive for the adaptive integrators (rk45, dop853)

意味
    適応積分器の ``sim.rtol``、``sim.atol``、``sim.output_interval`` が正でない。

主な原因
    0 や負の値。

対処
    正の値にする。

.. _IY-S080:

IY-S080
-------

重大度: エラー

メッセージ::

    error[IY-S080]: rocket never left the launch rail (thrust too low?)

意味
    機体がレールを離れなかった。

主な原因
    推力が機体重量に足りない、レールが長すぎる、仰角が高すぎる。

対処
    推力と質量、``launch.rail_length`` を確認する。

.. _IY-S081:

IY-S081
-------

重大度: エラー

メッセージ::

    error[IY-S081]: integration diverged at t = <t> s; reduce sim.dt or tighten rtol/atol, or check the aero table and mass properties

意味
    積分が発散した。

主な原因
    刻みが大きすぎる、空力表や質量特性が不適切である。

対処
    ``sim.dt`` を小さくするか許容誤差を締める。空力表と質量特性を確認する。

関連
    :ref:`page-theory-dynamics`

.. _IY-S090:

IY-S090
-------

重大度: エラー

メッセージ::

    error[IY-S090]: did not land within sim.max_time

意味
    サンプルが ``sim.max_time`` までに着地しなかった。

主な原因
    降下が遅い、開傘しなかった、``sim.max_time`` が短い。

対処
    ``sim.max_time`` を延ばすか降下条件を確認する。この文字列は ``dispersion_mc.csv`` の status にも書かれる。

関連
    :ref:`sec-mc-failure`

.. _IY-S500:

IY-S500
-------

重大度: ワーニング

メッセージ::

    warning[IY-S500]: the thrust curve has a single point: the thrust ramps linearly from 0 N at t = 0

意味
    推力曲線が 1 点だけで、0 N から直線で立ち上がると解釈した。

主な原因
    推力曲線の点が足りない。

対処
    推力曲線の点を加える。

.. _IY-S501:

IY-S501
-------

重大度: ワーニング

メッセージ::

    warning[IY-S501]: thrust curve ends at <last> N instead of 0 N: the motor is cut off abruptly

意味
    推力曲線の最後の推力が 0 N でなく、推力が急に切れる。

主な原因
    推力曲線の末尾に 0 N の点がない。

対処
    末尾に推力 0 N の点を加える。

.. _IY-S502:

IY-S502
-------

重大度: ワーニング

メッセージ::

    warning[IY-S502]: burn time <value> s is shorter than 0.05 s: the motor is an impulse; check the time column

意味
    燃焼時間が 0.05 秒より短く、インパルスとして扱われる。

主な原因
    時刻列の単位の誤り（ms で書いた等）。

対処
    時刻列を確認する。

