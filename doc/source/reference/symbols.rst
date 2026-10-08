.. _page-reference-symbols:

記号と単位
============

空力係数と座標系で使う記号、符号の規約、単位を引く。導出と図は :ref:`page-theory-coordinates` と :ref:`page-theory-aero-coefficients` にある。

.. _sec-symbols-coordinates:

座標系と符号の規約
------------------

.. list-table::
   :header-rows: 1
   :widths: 24 76

   * - 項目
     - 規約
   * - 局所 ENU 座標系
     - 原点は射点。:math:`x_E` が東、:math:`y_N` が北、:math:`z_U` が天頂（射点での楕円体の法線方向）の右手系。:math:`z_U` は射点からの高さで、海抜高度は :math:`h_0+z_U`\ （:math:`h_0` は射点標高）。
   * - 機体座標系
     - 原点は重心。:math:`x_b` は機軸に沿ってノーズ方向、:math:`y_b` と :math:`z_b` はそれに直交する右手系。向きは STL から決まる（``geometry.json`` の ``body_x`` ``body_y`` ``body_z``。:ref:`sec-body-basis`）。
   * - 機体上の位置
     - ノーズ先端から後方へ測った距離 :math:`x` [m]。機体上の点 :math:`x` の機体座標は :math:`(x_{cg}-x,\,0,\,0)`。
   * - 迎角 :math:`\alpha`
     - 対気速度の機体座標の成分 :math:`(u_x,u_y,u_z)` に対して :math:`\alpha=\operatorname{atan2}(u_z,u_x)`、範囲は :math:`(-180^\circ,180^\circ]`。
   * - 横滑り角 :math:`\beta`
     - :math:`\beta=\arcsin(u_y/V)`、範囲は :math:`[-90^\circ,90^\circ]`。:math:`V` は対気速度の大きさ。
   * - 全迎角 :math:`\alpha_t`
     - 機軸と対気速度のなす角。:math:`\cos\alpha_t=\cos\alpha\cos\beta`、範囲は 0 から :math:`180^\circ`。
   * - 力の符号
     - :math:`C_X<0` が抗力の向き。:math:`C_A=-C_X`。
   * - モーメントの基準点
     - ノーズ先端まわり（``moment_ref = "nose_tip"``）。:math:`qSd` で無次元化する。
   * - 静安定余裕
     - :math:`(x_{cp}-x_{cg})/d` [cal]。正が安定。
   * - ロールの符号
     - 正のフィンのカントは :math:`+x_b` まわりの正のロールモーメントを生む。ランチャ上のロール角 0 では :math:`+y_b` が水平。
   * - 方位
     - 射方位は北から時計回り。風向は風が吹いてくる方位（北から時計回り）。

.. _sec-symbols-aero:

空力の記号
----------

.. list-table::
   :header-rows: 1
   :widths: 24 76

   * - 記号
     - 意味
   * - :math:`M`
     - Mach 数。係数表の第一の引数。
   * - :math:`\alpha_t`
     - 全迎角。軸対称な機体は向きによらずこの角だけで決まる。
   * - :math:`\alpha,\ \beta`
     - 機体座標での迎角と横滑り角。符号と向きを持ち、一般表の引数になる。:math:`\beta` は圧縮性の因子 :math:`\sqrt{|1-M^2|}` や衝撃波角にも同じ文字を使う（式のある章で断る）。
   * - :math:`C_N,\ C_A`
     - 機軸に垂直な法線力と、機軸方向の軸力の係数。軸対称表が持つ。
   * - :math:`C_X,C_Y,C_Z`
     - 機体座標 :math:`x_b,y_b,z_b` 方向の力の係数。一般表が持つ。:math:`C_N` は :math:`C_Y,C_Z` の合成にあたる。
   * - :math:`C_l,C_m,C_n`
     - :math:`x_b,y_b,z_b` まわりのロール、ピッチ、ヨーのモーメントの係数。ノーズ先端まわりで定義する。
   * - :math:`x_{cp}`
     - 圧力中心。法線力の作用点（ノーズ先端から後方へ測る）。軸対称な機体で意味を持つ。
   * - :math:`x_{cg}`
     - 重心（ノーズ先端から後方へ測る）。
   * - :math:`C_{N\alpha}`
     - 法線力係数の傾き [1/rad]。
   * - :math:`C_{lp},\ C_{mq},\ C_{nr}`
     - ロール、ピッチ、ヨーの角速度に対する減衰の微係数。
   * - :math:`C_{l\delta}`
     - フィンのカント 1 rad あたりのロールモーメント係数 [1/rad]。
   * - :math:`d`, :math:`S`
     - 基準直径 [m] と基準面積 [m\ :sup:`2`]（``aero_table.json`` の ``ref_diameter`` と ``ref_area``）。
   * - :math:`q`
     - 動圧 [Pa]。
   * - :math:`\hat p`
     - 無次元ロール角速度 :math:`pd/(2V)`。

.. list-table:: 軸対称表と一般表の係数の対応
   :header-rows: 1
   :widths: 24 24 52

   * - 軸対称表
     - 一般表
     - 関係
   * - 法線力 :math:`C_N`
     - :math:`C_Y,\ C_Z`
     - :math:`C_N` は機軸に垂直な力の大きさ。:math:`(C_Y,C_Z)` はその力を機体座標の成分で表したもの（:ref:`sec-aero-cn-cz`）。
   * - 軸力 :math:`C_A`
     - :math:`C_X`
     - :math:`C_X=-C_A` が前向き飛行の関係。符号は抗力の向きで逆になる。
   * - 圧力中心 :math:`x_{cp}`
     - :math:`C_m,\ C_n`
     - :math:`x_{cp}` は力の作用点の位置。:math:`C_m,C_n` はノーズ先端まわりのモーメントで、一般表では 1 個の圧力中心では表せない。

.. _sec-symbols-units:

単位
----

.. list-table::
   :header-rows: 1
   :widths: 30 70

   * - 量
     - 単位
   * - 長さ、位置、高度
     - m（位置はノーズ先端から後方へ）
   * - 質量
     - kg
   * - 慣性モーメント
     - kg m\ :sup:`2`
   * - 角度
     - deg（設定ファイルと CSV の列名が ``_deg`` のもの）。内部の傾きの係数は 1/rad
   * - 時間
     - s
   * - 速度
     - m/s
   * - 圧力、動圧
     - Pa
   * - 温度
     - K
   * - 静安定余裕
     - cal（基準直径の倍数）
   * - 抗力面積 :math:`C_DS`
     - m\ :sup:`2`
   * - メモリ
     - GB（``resources.memory_gb``）、byte（``cfd_report.json``）
