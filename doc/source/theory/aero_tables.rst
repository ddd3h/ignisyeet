.. _page-theory-aero-tables:

係数表
========

どの空力手法も、結果を Mach 数と迎角の格子上の **係数表** にまとめる。飛翔計算は、この表を内挿して空気力を求める。
本章は、表の種類（軸対称表と一般表）、格子の範囲と刻み、範囲の外の扱いを決める根拠を述べる。
表の形式（CSV の列と付随ファイル）は :ref:`sec-aero-table-format` にある。
手法の選び方は :ref:`page-theory-aero-choose`、係数の定義は :ref:`page-theory-aero-coefficients` にある。

この章で決めること
--------------------

* 係数表の種類。設定 ``aero.table_kind``\ （:ref:`cfg-aero-table_kind`）で、軸対称表（``"axisymmetric"``、既定）か一般表（``"general"``）を選ぶ。
* 格子の範囲と刻み。``aero.mach_max``、``aero.mach_step``、``aero.alpha_max_deg``、``aero.alpha_step_deg``、``aero.general_mach_step``。
* 格子の外を引いたときの外挿。``aero.extrapolation``。

選び方
--------

表の種類
~~~~~~~~~~

表の種類は、機体の形と、表を作る手法で決まる。:numref:`fig-choose-table-kind` の質問に上から答える。

.. _fig-choose-table-kind:

.. figure:: /_generated/tikz/table_kind_choice.*
   :width: 100%

   ``aero.table_kind`` の選び方（左）と、二つの表が保持する成分（右）。
   部品積み上げ法とパネル法は軸対称な機体の係数しか出さないので、一般表にしても軸対称表を展開するだけで、情報は増えない。
   非対称な形状の係数は、CFD が一般表として直接作る。

.. _tbl-choose-table-kind:

.. list-table:: 表の種類の選択
   :header-rows: 1
   :widths: 26 20 54

   * - 状況
     - 表の種類
     - 理由と注意
   * - 軸対称な機体を ``barrowman`` か ``panel`` で解く
     - 軸対称表（既定）
     - 全迎角 :math:`\alpha_t` だけで係数を引ける。一般表に展開しても飛翔はほぼ変わらない（サンプル機体で頂点高度の差 0.05 m）
   * - 6 成分の式で飛翔させ、ピッチ面とヨー面の静安定余裕を別に見たい
     - 一般表（展開）
     - 静安定余裕を面ごとに評価する。サンプル機体では最小余裕が軸対称表で 1.99 cal、一般表で 1.89 cal で、差は評価に使う迎角の区間の違いによる（:ref:`sec-vaero-table`）
   * - ポッド、ランチラグ、非対称なフィン、横滑りの連成を見たい
     - 一般表（``cfd``）
     - ``cfd`` の ``surface = "stl"`` か ``"step"``、``shape_symmetry`` が ``"pitch_plane"`` か ``"none"``、``betas_deg`` に 0 以外があるときは、CFD が一般表を作る（:ref:`sec-cfd-table-kind`）
   * - 外部の表を使う
     - 表の形式に合わせる
     - 軸対称な外部の表は、``table_kind = "general"`` で一般表に展開できる。外部の表が一般表ならそのまま使う

``aero.table_kind`` が効くのは、軸対称表を作る手法（``barrowman``、``panel``、``table``）だけである。
``cfd`` が作る表の種類は、``aero.cfd`` の設定（形状の与え方、対称性、横滑り角）で決まる。

格子の範囲と刻み
~~~~~~~~~~~~~~~~

格子は、飛翔が引く条件を覆い、条件の変化を線形内挿で追えるだけ細かくする。:numref:`tbl-choose-table-grid` に既定値と選び方をまとめる。

.. _tbl-choose-table-grid:

.. list-table:: 格子の範囲と刻み
   :header-rows: 1
   :widths: 20 14 66

   * - キー
     - 既定
     - 選び方
   * - ``mach_max``
     - 3
     - 飛翔の最大 Mach 数より大きくする。表が手前で終わると、結果は使えない（下記）。10 以下
   * - ``mach_step``
     - 0.02
     - 遷音速の抗力の立ち上がりを追える細かさにする。0.1 を超えると警告する（:ref:`IY-O500`）。部品積み上げ法の遷音速の補間帯の幅は 0.6、パネル法の既定の帯の幅は 0.4 なので、刻み 0.1 は帯を 6 区間と 4 区間に分ける
   * - ``alpha_max_deg``
     - 90
     - 既定のままにする。頂点のあとの弾道落下は機体が横向き（90° 近く）になり、軸対称表の飛翔は全迎角を 0〜90° に折り返して引く。90 より小さくすると結果は使えないことがある。180 以下
   * - ``alpha_step_deg``
     - 1
     - 法線力と圧力中心を線形に内挿できる細かさにする。5° を超えると警告する（:ref:`IY-O501`）
   * - ``general_mach_step``
     - 0.05
     - 部品積み上げ法で一般表を作るときだけ効く。パネル法と外部の表は軸対称表の刻みで展開し、CFD の一般表は ``aero.cfd.general_mach_step``\ （既定 0.1、0 のとき ``mach_step``）で決める

既定の格子は Mach 0〜3 を 0.02 刻み（151 点）、迎角 0〜90° を 1° 刻み（91 点）で、13 741 点になる。
格子の点の数（Mach 数の数 × 迎角の数）は 10\ :sup:`6` 以下で、超えるとエラーになる。
一般表は、迎角 :math:`-180^\circ` から :math:`180^\circ`、横滑り角 :math:`-90^\circ` から :math:`90^\circ` の全範囲を持つので、同じ刻みでも点が多い。

外挿の選び方
~~~~~~~~~~~~

迎角と横滑り角が格子の外に出たときの扱いは、``aero.extrapolation`` で選ぶ。

* ``"linear"``\ （既定）：端の区間の傾きを延ばす。下降中の機体は大きな迎角をとるので、既定は線形である。符号と範囲は物理的に妥当な値に制限する。
* ``"clamp"``：端の値で一定とする。

どちらを選んでも、表の範囲外の引き出しがあった飛翔は、結果を使えないものとして扱う（:ref:`sec-aero-out-of-range`）。
Mach 数は、設定にかかわらず端の値で一定とする。

モデル
--------

.. _sec-aero-table:

軸対称表と一般表
~~~~~~~~~~~~~~~~

係数表には、軸対称表と一般表の 2 種類がある。
軸対称表は、機体が軸対称であることを前提に、Mach 数と全迎角 :math:`\alpha_t` だけで係数を引く表である。
一般表は、機体座標の 6 成分の力とモーメントを、Mach 数・迎角 :math:`\alpha`・横滑り角 :math:`\beta` の関数として持つ。
一般表は 6 成分の準定常空力（:math:`C_X,C_Y,C_Z,C_l,C_m,C_n` を :math:`(M,\alpha,\beta)` の関数として持つ）であり、
角速度によるモーメントの変化は、減衰の項で別に加える。非定常な空力をすべて表すものではない（:ref:`sec-aero-damping`）。
どちらの表を使うかは、設定 ``aero.table_kind``\ （既定は ``"axisymmetric"``）で選ぶ（:numref:`fig-aero-tables`）。

.. _fig-aero-tables:

.. figure:: /_generated/tikz/aero_tables.*
   :width: 100%

   軸対称表（左）と一般表（右）。部品積み上げ法とパネル法の一般表は、軸対称表を展開して作る。CFD の一般表と外部の一般表は、展開を通さずに直接作る。

軸対称表は、既定では Mach 数 0 から 3、全迎角 0 から 90° の格子を持つ。
全迎角が機体の軸からのずれの大きさだけを表すので、機体が軸対称なら十分である。
飛翔計算では、全迎角に対して :math:`\alpha_t'=\min(\alpha_t,\pi-\alpha_t)`、つまり 0 から 90° の範囲で表を引く。
既定の ``alpha_max_deg = 90`` で、頂点のあとの弾道落下で現れる大きな迎角（機体が横向きになる 90° 近く）も表の範囲に収まる。
``alpha_max_deg`` を 90 より小さくした表でその範囲を超えて引くと、結果を使えないものとして終了ステータス 3 で終わる（:ref:`sec-aero-out-of-range`）。
後ろ向きに飛ぶ（:math:`\alpha_t>90^\circ`）ときは、前向きの係数を引いて軸力の向きを反転する（:ref:`sec-aero-forces`）。

一般表は、迎角 :math:`-180^\circ` から :math:`180^\circ`、横滑り角 :math:`-90^\circ` から :math:`90^\circ` の全範囲を格子で覆う。
後ろ向きの飛行も表の一部として持つ。部品積み上げ法とパネル法は軸対称な機体の係数しか出さないので、一般表はそれを展開して作る。
全迎角は :math:`\cos\alpha_t=\cos\alpha\cos\beta`\ （:ref:`sec-aero-angles`）から求め、軸対称表を :math:`\alpha_t'` で引いて、
:eq:`eq-coeff-6-axisym` で 6 係数にする。
したがって、展開した表の飛翔は軸対称表の飛翔とほぼ一致する。展開した表は軸対称な機体以上の情報を持たない。
非対称な機体の係数は、CFD の一般表（:ref:`sec-cfd-table-kind`）か外部の一般表から与える。
軸対称表の :math:`\alpha_t'` と :math:`C_N,x_{cp}` の関係、6 成分の定義は :ref:`page-theory-aero-coefficients` で述べる。

表のファイル（CSV と、同名で拡張子が ``.json`` の付随ファイル）の列、付随ファイルの項目、バージョンは :ref:`sec-aero-table-format` に書いてある。
軸対称表の CSV の ``alpha_deg`` は :math:`\alpha_t`、一般表では :math:`\alpha` である。

**カントの基準**

フィンにカントがあると、ロールのモーメントが生じる。表のロールの強制項が、どのカントの値で作られているかを、付随ファイルの ``fin_cant_ref_deg``\ （:math:`\delta_{\mathrm{ref}}`、既定は 0）に書く。
飛翔計算は、設定したカント :math:`\delta` が :math:`\delta_{\mathrm{ref}}` と違うときだけ、その差の分を足す。式は :eq:`eq-cl-total` である（:ref:`sec-roll-moment`）。
部品積み上げ法とパネル法は設定したカントで表を作るので :math:`\delta_{\mathrm{ref}}=\delta` で、強制項を二重に数えることはない。

.. _sec-aero-interp:

係数表の内挿と外挿
~~~~~~~~~~~~~~~~~~

飛翔計算は、飛翔中の条件が表の格子点の間に入ったとき、近くの格子点の値から重み付きの和で係数を求める。
軸対称表は :math:`(M,\alpha_t)` の二次元、一般表は :math:`(M,\alpha,\beta)` の三次元の線形内挿で、軸が不等間隔でも使える。
格子の範囲の外では、迎角と横滑り角は端の区間の傾きをそのまま延ばす線形外挿が既定で、端の値で一定とすることもできる。
Mach 数は、設定にかかわらず、表の端の値で一定とする。

**双線形内挿**

軸対称表の飛翔中の :math:`(M,\alpha_t)` が格子の区間 :math:`[M_i,M_{i+1}]\times[\alpha_j,\alpha_{j+1}]` にあるとき（:numref:`fig-table-interp`）、

.. math::

   t_M = \frac{M-M_i}{M_{i+1}-M_i},\qquad t_\alpha=\frac{\alpha-\alpha_j}{\alpha_{j+1}-\alpha_j}

として、四隅の値 :math:`c_{00},c_{10},c_{01},c_{11}` から

.. math::
   :label: eq-bilinear

   c(M,\alpha) = (1-t_M)(1-t_\alpha)\,c_{00} + t_M(1-t_\alpha)\,c_{10} + (1-t_M)\,t_\alpha\,c_{01} + t_M\,t_\alpha\,c_{11}

を求める。

圧力中心 :math:`x_{cp}` は、それ自体を内挿しない。法線力 :math:`C_N` と、ノーズ先端まわりのモーメント :math:`C_Nx_{cp}` をそれぞれ :eq:`eq-bilinear` で内挿し、

.. math::
   :label: eq-xcp-interp

   x_{cp}(M,\alpha)=\frac{\bigl(C_Nx_{cp}\bigr)(M,\alpha)}{C_N(M,\alpha)}

とする。飛翔に効くモーメント :math:`C_Nx_{cp}` は内挿したモーメントそのものになり、同じ格子に展開した一般表（:math:`C_Z` と :math:`C_m` を別々に内挿する）と一致する。
内挿した :math:`C_N` が 0 のとき（迎角 0）は、迎角 0 の傾きについて同じ比 :math:`\bigl(C_{N\alpha}x_{cp}\bigr)/C_{N\alpha}` を使い、:math:`C_{N\alpha}` も 0 なら表の :math:`x_{cp}` をそのまま内挿する。

.. _fig-table-interp:

.. figure:: /_generated/tikz/table_interp.*
   :width: 70%

   係数表の双線形内挿と、範囲外の扱い。迎角（一般表では横滑り角も）の範囲外は端の区間の線形外挿で、Mach 数は範囲外でも表の端の値にクランプし、外挿しない。

**三線形内挿**

一般表は、:math:`(M,\alpha,\beta)` が入る格子の直方体の 8 隅の値から三線形内挿で求める。
:math:`\beta` 方向の分率を :math:`t_\beta` として、\ :eq:`eq-bilinear` の :math:`c_{00},\dots,c_{11}` を :math:`\beta_k` と :math:`\beta_{k+1}` の値の線形内挿
:math:`(1-t_\beta)c_{ij}^{(k)}+t_\beta c_{ij}^{(k+1)}` に置き換えればよい。
区間は軸が不等間隔でも、軸の値の並びから探す。

**外挿**

**Mach 数** は、設定にかかわらず、表の最小値より小さいときは最小値の、最大値より大きいときは最大値の係数を使う（端の値で一定）。
遷音速や超音速の係数の傾向は、表の範囲の外で直線的には続かないので、Mach 数の方向には外挿しない。
端の値で一定とした係数は飛翔の Mach 数の係数ではないので、範囲外の引き出しがあった飛翔の結果は使えないものとする（:ref:`sec-aero-out-of-range`）。

**迎角と横滑り角** が表の格子の外にあるときは、設定 ``extrapolation`` に従う。
CFD の表の、解いた迎角より大きい行は、この外挿ではなく部品積み上げ法へのつなぎで埋める（:ref:`page-theory-cfd-coefficients`）。``extrapolation`` が効くのは、表の格子の外の引き出しだけである。
``"linear"`` は、最も近い端の区間を使い、:math:`t_\alpha` が :math:`[0,1]` を外れたまま :eq:`eq-bilinear` を評価する（端の区間の傾きをそのまま延長する **線形外挿**）。
``"clamp"`` は :math:`t_\alpha` を :math:`[0,1]` に制限し、端の値で一定とする。

線形外挿した値は、物理的に妥当な符号と範囲に制限する。端の値（外挿しない値）を基準に、次のとおりとする。

* 法線力係数 :math:`C_N`、ノーズ先端まわりのモーメント :math:`C_Nx_{cp}`、迎角 0 のモーメントの傾き :math:`C_{N\alpha}x_{cp}`、軸力係数 :math:`C_A`\ （推力あり・なし）、ロールの強制項 :math:`C_{l\delta}` は、端の値の符号を保つ（符号が反転する分は 0 にする）。
* 法線力係数の傾き :math:`C_{N\alpha}` と、ピッチ減衰の三つの和は 0 以上とする。
* 外挿した値から :eq:`eq-xcp-interp` で求めた圧力中心 :math:`x_{cp}` は、機体の内側（ノーズ先端から全長まで）に収める。
* ロール減衰 :math:`C_{lp}` は、外挿の有無にかかわらず、正にしない（0 以下）。

一般表では、端の値の符号を保つ規則を、軸力、横力 :math:`C_Y`、法線方向の力 :math:`C_Z`、モーメント :math:`C_m` と :math:`C_n`、ロールの強制項（基準のカント角での :math:`C_l` と :math:`C_{l\delta}`）、減衰の和（ピッチとヨーの各 3 個）に適用し、減衰の和は 0 以上とする。端に向かって小さくなる横方向の力とモーメントを外挿しても、符号が反転しない。:math:`C_{lp}` は 0 以下に制限する。
:math:`\alpha,\beta` の軸が全範囲を覆うので、一般表で外挿が起こるのは、表の軸が全範囲に満たないときだけである。

.. _sec-aero-external:

外部の係数表
~~~~~~~~~~~~

風洞試験や他の解析で得た係数は、``aero.method = "table"`` で読み込める。
形状の抽出と空力モデルの計算は行わず、``sim`` と ``dispersion`` は読み込んだ表をそのまま使う。
表の形式は、このソフトが書く ``aero_table.csv`` と同じである（列と付随ファイルの項目は :ref:`sec-aero-table-format`）。
軸対称な外部の表を ``table_kind = "general"`` で一般表に展開することも、外部の表がすでに一般表ならそのまま使うこともできる。
外部の表を用意する手順は :ref:`sec-usage-external-table` にある。

表のロール項（:math:`C_{l\delta}` と :math:`C_{lp}`）がすべて 0 のときは、現在の STL から抽出したフィンの寸法で部品積み上げ法のロール項を求めて表に書き込む
（:ref:`sec-barrowman-roll`）。このとき端末に注意書きを表示する。形状を抽出できなければ、注意書きを表示してロール項は 0 のままになり、機体はロールしない。
フィンがなければ書き込む値も 0 である。ロール項を含む表は、そのまま使う。

適用範囲と限界
----------------

.. _sec-aero-out-of-range:

表の範囲外の引き出し
~~~~~~~~~~~~~~~~~~~~

飛翔が表の範囲の外の条件を引くと、結果は使えないものとして扱う。これは警告ではなく、終了ステータス 3 の結果である（:ref:`sec-exit-status`）。

.. list-table:: 範囲外の引き出しと結果
   :header-rows: 1
   :widths: 26 30 44

   * - 範囲外の条件
     - 終了と記録
     - 理由と対処
   * - Mach 数が表の最大値を超える
     - ``RESULTS NOT USABLE``\ （:ref:`IY-O022`）。``aero_table_range`` の ``machs_in_range`` が ``false``
     - 表が飛翔の Mach 数より手前で終わると、端の値で一定とした係数で上昇全体の抗力と安定が変わる。``aero.mach_max`` を上げて作り直す
   * - Mach 数が表の最小値を下回る
     - ``RESULTS NOT USABLE``\ （:ref:`IY-O023`）
     - 外部の表が Mach 0 から始まらないときに起こる。表を Mach 0 まで広げる
   * - 迎角（一般表では迎角または横滑り角）が格子の外
     - ``RESULTS NOT USABLE``\ （:ref:`IY-O024`）。``angles_in_range`` が ``false``
     - 外挿した係数は、圧力中心を重心より前へ動かして見かけの釣り合い（トリム）を作るなど、弾道落下の経路を変えうる。外挿の結果が範囲内の結果に近いかどうかは、機体と条件で決まり、事前には分からない。表の上限を 90° 以上にして作り直す

軸対称表では、飛翔計算が全迎角を 90° 以下に折り返して引くので、``alpha_max_deg`` が 90 以上なら迎角の範囲外は起こらない。
範囲外の引き出しがなくても、``summary.json``、``dispersion_meta.json``、``dispersion_summary.json`` の ``aero_table_range`` に、全引き出しの回数、範囲外の回数、要求された最大の Mach 数と迎角を記録する（:ref:`sec-file-summary`）。
迎角の最大値は、軸対称表では引いた全迎角、一般表では :math:`\arccos(\cos\alpha\cos\beta)` で、ピッチ面の迎角だけではない。

表の仮定
~~~~~~~~

* 軸対称表は機体が軸対称であることを前提にする。非対称な機体の係数を含まない。
* 展開した一般表は、軸対称表と同じ情報しか持たない。非対称な機体の係数は、CFD の一般表か外部の一般表から与える。
* 一般表は 6 成分の準定常空力で、完全な非定常空力ではない。角速度についての交差微係数は表せない（:ref:`sec-lim-motion`）。
* 遷音速の係数は、部品積み上げ法とパネル法では補間値である（:ref:`page-theory-aero-choose`）。

仮定と適用範囲の全体は :ref:`sec-lim-aero` にある。

精度の証拠
------------

係数表の読み込みと内挿・外挿は、軸対称表と一般表の両方で検証している（:ref:`sec-vaero-table`）。

* 軸対称表を展開した一般表は、軸対称表と同じ空気力とモーメントを与える。
* 展開した一般表の飛翔は、軸対称表の飛翔とほぼ一致する。サンプル機体の頂点高度は、軸対称表で 4440.48 m、一般表で 4440.52 m である。
* 保存した表を読み込み直しても、係数は :math:`10^{-8}` 以内で一致する。

これらはモデルが式どおりに動くことの検証であり、実測との比較ではない。係数そのものの精度は、表を作った手法で決まる（:ref:`sec-aero-accuracy-measured`）。

関連する設定キー
------------------

.. list-table::
   :header-rows: 1
   :widths: 40 60

   * - キー
     - 本章で決めること
   * - :ref:`cfg-aero-table_kind`
     - 軸対称表か一般表か
   * - :ref:`cfg-aero-mach_max`、:ref:`cfg-aero-mach_step`
     - Mach 数の範囲と刻み
   * - :ref:`cfg-aero-alpha_max_deg`、:ref:`cfg-aero-alpha_step_deg`
     - 全迎角の範囲と刻み
   * - :ref:`cfg-aero-general_mach_step`
     - 部品積み上げ法の一般表の Mach 数の刻み
   * - :ref:`cfg-aero-extrapolation`
     - 迎角と横滑り角の格子の外の扱い
   * - :ref:`cfg-aero-table`
     - 外部の係数表のファイル
