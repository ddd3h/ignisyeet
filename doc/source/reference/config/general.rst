.. _page-reference-config-general:

基本の設定
============

各表のキーを、型、既定値、値の範囲、説明の順に引く。長さの単位は m、位置はノーズ先端から後方へ測る。
"必須" のキーは省略できない。

.. _cfg-output:

``[output]``
------------

結果ファイルの出力先と KML の有無を指定する。

.. _cfg-output-dir:

``output.dir``
^^^^^^^^^^^^^^

:型: パス
:既定値: ``"out"``
:説明: 結果ファイルを書き出すディレクトリ。相対パスは設定ファイルのあるディレクトリを基準にする。``--out`` で上書きできる。
:関連: :ref:`sec-cli-overrides`

.. _cfg-output-kml:

``output.kml``
^^^^^^^^^^^^^^

:型: 真偽値
:既定値: ``true``
:値/範囲: ``true`` ``false``
:説明: Google Earth 用の ``ignisyeet.kml`` を書き出すか。
:関連: :ref:`sec-kml`

.. _cfg-resources:

``[resources]``
---------------

計算資源の上限を指定する。すべて省略できる。結果は変えない。

.. _cfg-resources-threads:

``resources.threads``
^^^^^^^^^^^^^^^^^^^^^

:型: 整数
:既定値: ``0``
:値/範囲: 0 以上
:説明: 全処理で使う CPU スレッド数の上限。``0`` は使用可能な全スレッド。使用可能数を超える値は警告して使用可能数に切り詰める。落下分散、モンテカルロ、パネル法の行列計算、CFD の MPI ランク数に効く。

.. _cfg-resources-memory_gb:

``resources.memory_gb``
^^^^^^^^^^^^^^^^^^^^^^^

:型: 数値
:既定値: ``0``
:値/範囲: 0 以上（負の値はエラー）
:説明: 重い処理のメモリ予算 [GB]。``0`` はこのプロセスが使えるメモリ（Linux では ``MemAvailable``、cgroup の上限、``ulimit -v`` の上限の最小。macOS では物理メモリ）。
   パネル法は、密行列に必要な量（パネル数 :math:`n` に対して :math:`8n^2+64n` バイト）が予算を超えると、計算の前にエラーで止まる。
   CFD は、予算に収まるよう同時実行ケース数を減らす。1 ケースの見積もりが予算を超えるときは、メッシュ生成の後、計算の前にエラーで止まる。
:関連: :ref:`IY-A018`、:ref:`sec-cfd-resources`

.. _cfg-resources-nice:

``resources.nice``
^^^^^^^^^^^^^^^^^^

:型: 整数
:既定値: ``0``
:値/範囲: 0 以上 19 以下
:説明: プロセスの優先度を下げる値。SU2 や ``mpirun`` の子プロセスにも引き継がれる。Unix 以外のプラットフォームでは警告して無視する。

.. _sec-config-rocket:

.. _cfg-rocket:

``[rocket]``
------------

機体の STL と質量特性、表面とフィンの仕様を指定する。慣性テンソル、``cg_lateral_dry``、``misalignment_offset`` は機体座標（:math:`x_b` がノーズ方向）で与える（:ref:`sec-body-basis`）。

.. _cfg-rocket-stl:

``rocket.stl``
^^^^^^^^^^^^^^

:型: パス
:既定値: 必須
:説明: 機体の STL ファイル（ASCII とバイナリのどちらも読める）。
:関連: :ref:`page-theory-geometry`

.. _cfg-rocket-stl_scale:

``rocket.stl_scale``
^^^^^^^^^^^^^^^^^^^^

:型: 数値
:既定値: ``1.0``
:値/範囲: 正
:説明: STL の長さの単位を m に直す係数。mm の STL は ``0.001``。掛けた後の機体の全長は 0.1 m 以上 20 m 以下。
:関連: :ref:`page-theory-geometry`

.. _cfg-rocket-nose_direction:

``rocket.nose_direction``
^^^^^^^^^^^^^^^^^^^^^^^^^

:型: 文字列
:既定値: ``"auto"``
:値/範囲: ``"auto"`` ``"+x"`` ``"-x"`` ``"+y"`` ``"-y"`` ``"+z"`` ``"-z"``
:説明: STL 座標でノーズが向く方向。``"auto"`` は形状から判定する。
:関連: :ref:`page-theory-geometry`

.. _cfg-rocket-dry_mass:

``rocket.dry_mass``
^^^^^^^^^^^^^^^^^^^

:型: 数値
:既定値: 必須
:値/範囲: 正
:説明: 推進剤を除いた質量 [kg]。空のモータケースを含む。
:関連: :ref:`page-theory-propulsion`

.. _cfg-rocket-cg_dry:

``rocket.cg_dry``
^^^^^^^^^^^^^^^^^

:型: 数値
:既定値: 必須
:値/範囲: 0 以上、機体の全長以下
:説明: 推進剤を除いた重心のノーズ先端からの位置 [m]。
:関連: :ref:`page-theory-propulsion`

.. _cfg-rocket-ixx_dry:

``rocket.ixx_dry``
^^^^^^^^^^^^^^^^^^

:型: 数値
:既定値: 必須
:値/範囲: 正
:説明: 推進剤を除いたロール慣性モーメント [kg m\ :sup:`2`]（乾燥重心まわり、機体座標）。
:関連: :ref:`sec-usage-inertia-estimate`、:ref:`page-theory-propulsion`

.. _cfg-rocket-iyy_dry:

``rocket.iyy_dry``
^^^^^^^^^^^^^^^^^^

:型: 数値
:既定値: 必須
:値/範囲: 正
:説明: 推進剤を除いたピッチ慣性モーメント [kg m\ :sup:`2`]（乾燥重心まわり、機体座標）。
:関連: :ref:`sec-usage-inertia-estimate`、:ref:`page-theory-propulsion`

.. _cfg-rocket-izz_dry:

``rocket.izz_dry``
^^^^^^^^^^^^^^^^^^

:型: 数値
:既定値: ``iyy_dry``
:値/範囲: 正
:説明: 推進剤を除いたヨー慣性モーメント [kg m\ :sup:`2`]。省略すると ``iyy_dry`` に等しい。
:関連: :ref:`page-theory-propulsion`

.. _cfg-rocket-ixy_dry:

``rocket.ixy_dry``
^^^^^^^^^^^^^^^^^^

:型: 数値
:既定値: ``0``
:値/範囲: 有限
:説明: 慣性テンソルの非対角要素 :math:`I_{xy}=-\int xy\,\mathrm{d}m` [kg m\ :sup:`2`]（乾燥重心まわり、機体座標）。慣性テンソルは正定値でなければならない。CAD が慣性乗積を :math:`+\int xy\,\mathrm{d}m` で表示するときは符号を反転して与える。
:関連: :ref:`sec-inertia-tensor`

.. _cfg-rocket-ixz_dry:

``rocket.ixz_dry``
^^^^^^^^^^^^^^^^^^

:型: 数値
:既定値: ``0``
:値/範囲: 有限
:説明: 慣性テンソルの非対角要素 :math:`I_{xz}=-\int xz\,\mathrm{d}m` [kg m\ :sup:`2`]。符号の規約は ``ixy_dry`` と同じ。
:関連: :ref:`sec-inertia-tensor`

.. _cfg-rocket-iyz_dry:

``rocket.iyz_dry``
^^^^^^^^^^^^^^^^^^

:型: 数値
:既定値: ``0``
:値/範囲: 有限
:説明: 慣性テンソルの非対角要素 :math:`I_{yz}=-\int yz\,\mathrm{d}m` [kg m\ :sup:`2`]。符号の規約は ``ixy_dry`` と同じ。
:関連: :ref:`sec-inertia-tensor`

.. _cfg-rocket-cg_lateral_dry:

``rocket.cg_lateral_dry``
^^^^^^^^^^^^^^^^^^^^^^^^^

:型: 数値の配列（2 要素）
:既定値: ``[0, 0]``
:値/範囲: 有限
:説明: 乾燥重心の機軸からの横ずれ ``[y, z]`` [m]（機体座標）。
:関連: :ref:`sec-body-basis`

.. _cfg-rocket-fin_cant_deg:

``rocket.fin_cant_deg``
^^^^^^^^^^^^^^^^^^^^^^^

:型: 数値
:既定値: ``0``
:値/範囲: 有限
:説明: フィンのカント [deg]。正のカントは :math:`+x_b` まわりの正のロールモーメントを生む。``fin_override.cant_deg`` があればそちらを使う。
:関連: :ref:`page-theory-dynamics`

.. _cfg-rocket-roughness:

``rocket.roughness``
^^^^^^^^^^^^^^^^^^^^

:型: 数値
:既定値: ``60e-6``
:値/範囲: 0 以上
:説明: 表面の等価砂粒粗さ [m]。摩擦抗力に使う。
:関連: :ref:`page-theory-barrowman`

.. _cfg-rocket-fin_le:

``rocket.fin_le``
^^^^^^^^^^^^^^^^^

:型: 文字列
:既定値: ``"rounded"``
:値/範囲: ``"rounded"`` ``"sharp"``
:説明: フィンの前縁形状。丸め、または鋭角。
:関連: :ref:`page-theory-barrowman`

.. _cfg-rocket-fin_te:

``rocket.fin_te``
^^^^^^^^^^^^^^^^^

:型: 文字列
:既定値: ``"square"``
:値/範囲: ``"square"`` ``"tapered"``
:説明: フィンの後縁形状。切り落とし（底面抗力あり）、またはテーパ。
:関連: :ref:`page-theory-barrowman`

.. _cfg-rocket-extra_cd:

``rocket.extra_cd``
^^^^^^^^^^^^^^^^^^^

:型: 数値
:既定値: ``0.0``
:値/範囲: 0 以上
:説明: ランチャラグなど付加物の抗力係数。
:関連: :ref:`page-theory-barrowman`

.. _cfg-rocket-fin_override:

``[rocket.fin_override]``
-------------------------

``[rocket.fin_override]`` は、STL から検出したフィンの寸法を上書きする。書いたキーだけが置き換わり、省略したキーは検出した値のままである。

.. _cfg-rocket-fin_override-count:

``rocket.fin_override.count``
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

:型: 整数
:既定値: 検出値
:値/範囲: 0 以上
:説明: フィンの枚数。
:関連: :ref:`page-theory-geometry`

.. _cfg-rocket-fin_override-root_chord:

``rocket.fin_override.root_chord``
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

:型: 数値
:既定値: 検出値
:値/範囲: 0 以上
:説明: フィン根元の翼弦長 [m]。
:関連: :ref:`page-theory-geometry`

.. _cfg-rocket-fin_override-tip_chord:

``rocket.fin_override.tip_chord``
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

:型: 数値
:既定値: 検出値
:値/範囲: 0 以上
:説明: フィン先端の翼弦長 [m]。
:関連: :ref:`page-theory-geometry`

.. _cfg-rocket-fin_override-span:

``rocket.fin_override.span``
^^^^^^^^^^^^^^^^^^^^^^^^^^^^

:型: 数値
:既定値: 検出値
:値/範囲: 0 以上
:説明: フィンのスパン [m]（胴体表面から先端まで）。
:関連: :ref:`page-theory-geometry`

.. _cfg-rocket-fin_override-sweep:

``rocket.fin_override.sweep``
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

:型: 数値
:既定値: 検出値
:値/範囲: 有限
:説明: 前縁の後退の軸方向長さ [m]。
:関連: :ref:`page-theory-geometry`

.. _cfg-rocket-fin_override-thickness:

``rocket.fin_override.thickness``
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

:型: 数値
:既定値: 検出値
:値/範囲: 0 以上
:説明: フィンの厚さ [m]。``aero.method = "panel"`` と、``aero.method = "cfd"`` で ``aero.cfd.surface = "panel_mesh"`` のときは正。部品積み上げ法では抗力にだけ使う。
:関連: :ref:`page-theory-geometry`

.. _cfg-rocket-fin_override-x_le_root:

``rocket.fin_override.x_le_root``
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

:型: 数値
:既定値: 検出値
:値/範囲: 有限
:説明: フィン根元の前縁のノーズ先端からの位置 [m]。
:関連: :ref:`page-theory-geometry`

.. _cfg-rocket-fin_override-cant_deg:

``rocket.fin_override.cant_deg``
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

:型: 数値
:既定値: ``rocket.fin_cant_deg``
:値/範囲: 有限
:説明: フィンのカント [deg]。``rocket.fin_cant_deg`` の代わりに使う。
:関連: :ref:`cfg-rocket-fin_cant_deg`

.. _cfg-motor:

``[motor]``
-----------

モータの推力曲線と取り付けを指定する。

.. _cfg-motor-eng:

``motor.eng``
^^^^^^^^^^^^^

:型: パス
:既定値: 必須
:説明: RASP 形式（``.eng``）の推力曲線ファイル。
:関連: :ref:`page-theory-propulsion`、:ref:`sec-propulsion-eng`

.. _cfg-motor-aft_x:

``motor.aft_x``
^^^^^^^^^^^^^^^

:型: 数値
:既定値: 機体の全長
:値/範囲: 0 以上。モータの長さ以上、機体の全長以下（1 mm の許容がある）
:説明: モータ後端のノーズ先端からの位置 [m]。
:関連: :ref:`page-theory-propulsion`

.. _cfg-motor-nozzle_exit_diameter:

``motor.nozzle_exit_diameter``
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

:型: 数値
:既定値: ``0.0``
:値/範囲: 0 以上
:説明: ノズル出口の直径 [m]。燃焼中はこの面積だけ底面抗力が減る。
:関連: :ref:`page-theory-propulsion`

.. _cfg-motor-propellant_mass:

``motor.propellant_mass``
^^^^^^^^^^^^^^^^^^^^^^^^^

:型: 数値
:既定値: ``.eng`` の値
:値/範囲: 0 以上
:説明: 推進剤質量 [kg]。``.eng`` の値を上書きする。
:関連: :ref:`page-theory-propulsion`

.. _cfg-motor-misalignment_deg:

``motor.misalignment_deg``
^^^^^^^^^^^^^^^^^^^^^^^^^^

:型: 数値
:既定値: ``0``
:値/範囲: 有限
:説明: 推力の方向と機軸のなす角 :math:`\varepsilon` [deg]。
:関連: :ref:`sec-thrust-line`

.. _cfg-motor-misalignment_azimuth_deg:

``motor.misalignment_azimuth_deg``
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

:型: 数値
:既定値: ``0``
:値/範囲: 有限
:説明: 推力の傾きの向き :math:`\phi` [deg]。:math:`+y_b` から :math:`+z_b` へ測る。
:関連: :ref:`sec-thrust-line`

.. _cfg-motor-misalignment_offset:

``motor.misalignment_offset``
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

:型: 数値の配列（2 要素）
:既定値: ``[0, 0]``
:値/範囲: 有限
:説明: ノズル出口の点 ``[y, z]`` [m]（機体座標）。推力線はこの点を通る。
:関連: :ref:`sec-thrust-line`

.. _cfg-launch:

``[launch]``
------------

射点とランチャの条件を指定する。

.. _cfg-launch-latitude:

``launch.latitude``
^^^^^^^^^^^^^^^^^^^

:型: 数値
:既定値: 必須
:値/範囲: -90 以上 90 以下
:説明: 射点の緯度 [deg]（WGS84）。
:関連: :ref:`page-theory-atmosphere-gravity`

.. _cfg-launch-longitude:

``launch.longitude``
^^^^^^^^^^^^^^^^^^^^

:型: 数値
:既定値: 必須
:値/範囲: -180 以上 180 以下
:説明: 射点の経度 [deg]（WGS84）。
:関連: :ref:`page-theory-atmosphere-gravity`

.. _cfg-launch-altitude:

``launch.altitude``
^^^^^^^^^^^^^^^^^^^

:型: 数値
:既定値: 必須
:値/範囲: 有限
:説明: 射点の標高 [m]。
:関連: :ref:`page-theory-atmosphere-gravity`

.. _cfg-launch-rail_length:

``launch.rail_length``
^^^^^^^^^^^^^^^^^^^^^^

:型: 数値
:既定値: 必須
:値/範囲: 正
:説明: 機体が発射位置からレールに沿って進み、レールを離れるまでの距離 [m]（有効な案内長）。
:関連: :ref:`sec-rail`

.. _cfg-launch-elevation_deg:

``launch.elevation_deg``
^^^^^^^^^^^^^^^^^^^^^^^^

:型: 数値
:既定値: 必須
:値/範囲: 0 より大きく 90 以下
:説明: 射角（水平面からの仰角）[deg]。
:関連: :ref:`sec-initial-attitude`

.. _cfg-launch-azimuth_deg:

``launch.azimuth_deg``
^^^^^^^^^^^^^^^^^^^^^^

:型: 数値
:既定値: 必須
:値/範囲: 有限
:説明: 射方位 [deg]。北から時計回り。
:関連: :ref:`sec-initial-attitude`

.. _cfg-launch-roll_deg:

``launch.roll_deg``
^^^^^^^^^^^^^^^^^^^

:型: 数値
:既定値: ``0``
:値/範囲: 有限
:説明: ランチャ上の機体のロール角 [deg]。機軸まわりに回す。0 では :math:`+y_b` が水平である。
:関連: :ref:`sec-initial-attitude`

.. _cfg-recovery:

``[recovery]``
--------------

回収系（パラシュート）を指定する。

.. _cfg-recovery-enabled:

``recovery.enabled``
^^^^^^^^^^^^^^^^^^^^

:型: 真偽値
:既定値: 必須
:値/範囲: ``true`` ``false``
:説明: パラシュートを使うか。``false`` のとき落下分散は弾道落下だけを計算する。
:関連: :ref:`page-theory-descent`

.. _cfg-recovery-cd_s:

``recovery.cd_s``
^^^^^^^^^^^^^^^^^

:型: 数値
:既定値: 必須
:値/範囲: 0 以上（``enabled = true`` のときは正）
:説明: パラシュートの抗力面積 :math:`C_D S` [m\ :sup:`2`]。二段開傘ではドローグの値。
:関連: :ref:`page-theory-descent`

.. _cfg-recovery-delay:

``recovery.delay``
^^^^^^^^^^^^^^^^^^

:型: 数値
:既定値: 必須
:値/範囲: 0 以上
:説明: 頂点から開傘までの遅れ [s]。
:関連: :ref:`page-theory-descent`

.. _cfg-recovery-main_cd_s:

``recovery.main_cd_s``
^^^^^^^^^^^^^^^^^^^^^^

:型: 数値
:既定値: なし
:値/範囲: 正
:説明: 二段開傘のメインの抗力面積 :math:`C_D S` [m\ :sup:`2`]。``main_altitude`` と同時に指定するか、両方とも省略する。メインの開傘高度で抗力面積が ``main_cd_s`` に置き換わる（足し合わせない）。モンテカルロ解析の ``parachute_cd_s_scale`` は ``cd_s`` と ``main_cd_s`` の両方に掛かる。
:関連: :ref:`sec-dual-deploy`

.. _cfg-recovery-main_altitude:

``recovery.main_altitude``
^^^^^^^^^^^^^^^^^^^^^^^^^^

:型: 数値
:既定値: なし
:値/範囲: 正
:説明: メインの開傘高度 [m]（射点からの高さ）。下降中にこの高さを横切った時刻に開く。ドローグの開傘時にすでにこの高さ以下なら、ドローグと同時に開く。``main_cd_s`` と同時に指定する。
:関連: :ref:`sec-dual-deploy`

