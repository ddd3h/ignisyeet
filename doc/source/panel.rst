パネル法による空力解析
======================

部品積み上げ法（:doc:`barrowman`）は、機体を「ノーズ・円筒・フィン」といった部品に分け、部品ごとの経験式を足し合わせて係数を求める。
計算が速く、形状の細部によらず安定した値が得られるが、機体の実際の表面形状は使っていない。
とくにフィンと胴体の干渉は、一つの補正係数で表すしかない。

本章では、抽出した形状から表面メッシュを作り、その上で流れを解く **パネル法** を説明する。
設定ファイルで ``aero.method = "panel"`` を選ぶと、他の手法と同じ形式の係数表（:ref:`sec-aero-table`）がパネル法で作られ、
飛翔計算は何も変えずにそれを使う。

パネル法とは
------------

パネル法は、機体の表面の圧力の分布を求め、そこから法線力、圧力中心、軸力、ピッチ減衰を作る方法である。
解くのは表面だけで、機体のまわりの空間を格子で覆う必要はない。

物体表面を小さな面（パネル）に分け、各パネルから流れに擾乱が出ているとみなす。
擾乱の元になるのは、表面から流体が湧き出す「わき出し」と、表面を境にして流れの向きが逆になる「二重わき出し」である :cite:`hesssmith,katz`\ 。
それぞれのパネルでの強さを、「機体の壁を流れが貫通しない」という条件を満たすように決める。
強さが決まれば、パネルのすぐ外側の流速が分かり、そこから圧力を求められる。
CFD は流れ場の全体を格子で覆うが、パネル法の未知数は表面にしかないので、数千のパネルで済む。
一方で、扱う流れは **非粘性・渦なしのポテンシャル流** に限られる。

IgnisYeet では、抽出した形状（:doc:`geometry`）から、回転体の胴体と台形のフィンを組み合わせた閉じた表面メッシュを作る。
STL の三角形をそのまま使うのではなく、胴体の半径分布とフィンの寸法から作り直した形状である。
ポテンシャル流の解法が使えるのは亜音速に限られる（超音速では方程式が双曲型になり、定式化は成り立たない :cite:`liepmann`\ ）。
そこで Mach 数の範囲ごとに手法を使い分ける（次節）。

この方法を選んだ理由は次の三つである。

* 実際の表面形状から、ノーズの形、胴体のくびれ、フィンの平面形、フィンと胴体の干渉が自然に入る。
* 解くのは Mach 数ごとの線形問題で、迎角はそれぞれの解の線形結合で表せる。係数表の格子を埋める計算は軽い（計算時間は :ref:`sec-vaero-timing` にある）。
* 部品積み上げ法とは仮定が独立なので、両者の差は推算の不確かさの目安になる。

どの手法をどの順に使うかは :ref:`sec-aero-select` にまとめてある。
解析解や既知の関係との比較とメッシュの収束は :ref:`sec-vaero-panel` に、部品積み上げ法との相互比較は :ref:`sec-vaero-compare-panel` に、計算時間は :ref:`sec-vaero-timing` にある。

.. _sec-panel-flow:

処理の流れ
----------

パネル法は、表面をパネルに分けることから始め、力とモーメントを得るまでを :numref:`fig-panel-flow` の 6 段階で進む。
先に、何が未知で、どの条件を課し、何が得られるかを順に見ておく。

.. _fig-panel-flow:

.. figure:: _generated/tikz/panel_flow.*
   :width: 100%

   パネル法の処理の流れと、Mach 数ごとの使い分け（Mach 数の境は既定値）。亜音速は 6 段階をすべて通り、
   超音速は連立方程式を解かずに各パネルの傾きから圧力を求め、遷音速は両側の値をつなぐ。

1. **表面をパネルに分ける。** 胴体・フィン・翼端と、後端を塞ぐ尾部フェアリングを、辺を共有する閉じた曲面の四角形・三角形パネルにする（:ref:`sec-panel-build`）。
2. **各パネルに未知量を置く。** パネルごとに、わき出しの強さ :math:`\sigma` と二重わき出しの強さ :math:`\mu` を割り当てる。
   ただし :math:`\sigma` は次の段階で形状と流れの向きだけから決まるので、本当の未知数は :math:`\mu` だけである。
3. **壁面を流れが貫通しない条件を課す。** この条件は :math:`\sigma` に直接入る。
   物体の内部のポテンシャルをゼロとおく Dirichlet 条件が、:math:`\mu` を決める方程式になる。
4. **連立方程式を解く。** 係数は形状と Mach 数だけで決まり、右辺は前方からの流れ、横からの流れ、機首まわりの回転の 3 通りである。
   一度の LU 分解で 3 通りをまとめて解き、迎角が変わっても解き直さない。
5. **表面の圧力を求める。** :math:`\mu` の表面での勾配から表面の流速を作り、Bernoulli の式で圧力係数 :math:`C_p` にする。
6. **力とモーメントにする。** 圧力に面積と向きを掛けて足し合わせ、法線力・圧力中心・ピッチ減衰の和を得る。

Mach 数による使い分け
~~~~~~~~~~~~~~~~~~~~~

亜音速では、上の 6 段階をそのまま通る。圧縮性は、機体を横方向に縮めた物体で非圧縮の問題を解く Göthert の変換で入れる。
解いた Mach 数の間は、Mach 数について滑らかに内挿する。

超音速では、ある点の圧力はその点の傾きにほぼ局所的に決まるので、連立方程式を解かない。
各パネルの流れに対する傾斜角から、斜め衝撃波や膨張波の関係を使って圧力を直接決める（局所傾斜法）。
段階 2 から 4 は使わず、任意の Mach 数と迎角で、段階 5 と 6 に当たる計算ができる。

遷音速は、どちらの手法も成り立たない。本実装は、両側の値を 3 次の補間でつなぐだけで、遷音速の物理は計算しない。

軸力は、全 Mach 数を通して、パネルで積分した表面摩擦と、部品積み上げ法の経験項（底面・フィン縁・ボートテール・追加抗力）で作る。
超音速ではノーズなどの造波抗力を、局所傾斜法の圧力から積分して使う。

.. list-table::
   :header-rows: 1
   :widths: 24 76

   * - Mach 数
     - 手法
   * - 亜音速（``subsonic_machs``、既定は :math:`0\ldots0.8`）
     - Morino の Dirichlet 型定式化 :cite:`morino` による定数強度のわき出し・二重わき出しパネル法。
       Göthert の変換 :cite:`gothert` で圧縮性を入れる。解いた Mach 数の間は Mach 数について滑らかに内挿する。迎角 15° から 30° の間で部品積み上げ法の大迎角の値へ移す（「非線形な法線力」）
   * - 超音速（``transonic[1]`` 以上、既定は :math:`1.2` 以上）
     - 局所傾斜法。各パネルが自分の傾きだけで圧力を決める（接線くさび、Prandtl–Meyer 膨張、修正 Newton、Taylor–Maccoll の接線円すい）
   * - 遷音速（``transonic[0]``\ （``subsonic_machs`` の最大がそれより低ければその値）から ``transonic[1]`` まで。既定は :math:`0.8` から :math:`1.2`）
     - 両側の値をつなぐ 3 次 Hermite 補間（法線力とモーメントは両端の値の間に収まるよう傾きを制限する）
   * - 軸力（全 Mach 数）
     - パネルで積分した表面摩擦、超音速の造波抗力、および部品積み上げ法の底面・フィン縁・ボートテール・追加抗力の項

IgnisYeet での扱い
------------------

係数表の各点 :math:`(M,\alpha)` について、上の三つの領域の値を組み合わせ、軸力を加える。
できた表は、部品積み上げ法と同じ ``aero_table.csv``/``.json`` に保存する。

``method = "panel"`` のとき ``aero_drag.csv`` には合計の抗力（``cd_off``, ``cd_on``）、:math:`C_{N\alpha}`、:math:`x_{cp}` だけが書かれ、
:doc:`barrowman` のような内訳の列はない（軸力が表面積分で得られるため）。
係数表のロール項（カントあたりのロールモーメント :math:`C_{l\delta}` とロール減衰 :math:`C_{lp}`）は、パネルの解からは求めず、
部品積み上げ法のモデル（:ref:`sec-barrowman-roll`）の値をそのまま書き込む。パネル法のメッシュはカントを反映しない。

また ``panel_cp.csv`` に、最も低い亜音速 Mach 数（既定では :math:`M=0`）、迎角 :math:`4^\circ` での表面圧力係数を保存する（:numref:`fig-panel-cp`）。

.. _sec-panel-build:

パネルの作り方
--------------

座標系は:doc:`barrowman` と同じで、:math:`x` はノーズ先端から後方への距離である。メッシュは :numref:`fig-panel-mesh` のようになる。

.. _fig-panel-mesh:

.. figure:: _generated/tikz/panel_mesh.*
   :width: 100%

   パネルモデルの側面図（概念図）。
   胴体のパネルは軸方向にコサイン間隔、フィンは翼弦方向にコサイン間隔・スパン方向に翼端寄りに細かく切る。
   後端の後ろに、底面を塞ぐ尾部フェアリングがある。フィン後縁から後流のパネルが後方へ伸びる。

胴体
~~~~

胴体は、半径の分布を軸方向と周方向に切って、四角形のパネルにする。軸方向の間隔は、曲率の大きい場所で細かくなるように取る。

胴体は回転体として扱い、抽出した半径分布 :math:`r(x)`\ （断面ごとの値を折れ線でつないだもの。先端は :math:`r=0`、後端は ``base_radius``）から作る。
軸方向は ``body_axial`` 分割で、ステーションの位置を

.. math::
   :label: eq-pn-cosine

   x_i = x_0 + (x_1-x_0)\,\frac{1}{2}\left(1-\cos\frac{\pi i}{n}\right),\qquad i=0,\ldots,n

とコサイン間隔にする。ノーズ先端と後端、肩のように曲率の大きい部分でパネルが細かくなる。
フィンがあるときは、フィンの根元前縁から後縁までを ``fin_chord`` 分割し（同じコサイン間隔）、残りのパネル数を前後の長さの比で配分する。
フィンの根元後縁が全長の 0.5 % 以内まで後端に近ければ、後縁を後端に一致させる（サンプル機体がこれに当たる）。
周方向は ``body_circ`` 個の節点で、フィンが :math:`N` 枚あるときは各扇形に :math:`n_c/N` 個ずつ割り当てる。
ノーズ先端は一点に縮退させ、その周りのパネルは三角形になる。

フィン
~~~~~~

フィンは厚みを持つ板として、上面・下面・翼端のパネルで作る。胴体との接合部にすきまができないよう、胴体側を切り抜いて閉じた面にする。

フィンは厚みを持たせる。断面は **両凸**\ （``fin_section = "biconvex"``）、すなわち前縁・後縁が尖り、中央が最も厚い放物線弧の組合せで、
根元翼弦 :math:`c_r` 上の位置 :math:`\xi=(x-x_{le})/c_r` での半厚みを

.. math::
   :label: eq-pn-biconvex

   h(\xi) = \frac{t}{2}\,4\xi(1-\xi)

とする（:math:`t` は最大厚み）。厚みはスパン方向に一定である。
フィン面の格子点は、根元翼弦をコサイン間隔、スパン方向を :math:`s_j=\sin(\pi j/2n_s)`\ （翼端に向けて細かく）で切り、
各位置で翼弦と後退を台形の寸法から決める。翼端には、上下の面をつなぐ半径方向を向いた帯状のパネル（翼端キャップ）を置く。

フィンと胴体の接合部で隙間ができないよう、根元は次のように切り抜く（:numref:`fig-panel-section`）。
各フィンの中心方位を :math:`\varphi_k=2\pi k/N+\pi/N` とする。胴体の周方向の節点には、各フィンの両側に 2 点ずつ根元用の節点を含め、
フィン根元の範囲では、この 2 点を :math:`\theta=\varphi_k\pm\arcsin\bigl(h(\xi)/r\bigr)` に置く。
つまり胴体表面から、フィン断面と同じ形のレンズ形の領域を切り取り、フィンの上面・下面のパネルがその縁に直接つながる。
前縁と後縁では :math:`h=0` なので 2 点は一つの節点に合流する。切り抜いた領域の胴体パネルは作らない。
こうして胴体・フィン・翼端キャップは辺を共有する **閉じた曲面**\ （どの辺もちょうど 2 枚のパネルが共有する）になる。
この性質は自動テストで確かめている（``mesh_is_watertight``）。

.. _fig-panel-section:

.. figure:: _generated/tikz/panel_section.*
   :width: 100%

   左: フィン位置の断面。胴体の周方向の節点と、フィン根元のレンズ形の切り抜き（根元の幅は誇張して描いている。実際の半角は :math:`\arcsin(t/2r)` 程度）。
   右: 両凸のフィン断面。前縁・後縁は尖っている。

パネルは四角形と三角形で、各パネルは 4 つの角点から次のように決める。面積は対角線の外積の大きさの半分、法線はその外積の向き（流体側が外向き）、
重心は角点の平均である。四角形はわずかにねじれていても、法線に垂直な平面に射影した平面多角形として扱う。

尾部フェアリング
~~~~~~~~~~~~~~~~

機体の後端の底面は、実際には剥離した流れ（死水域）に接している。これをそのまま開口として扱うと、Dirichlet 型の定式化は破綻する。
開いた表面では内側のポテンシャルが一意に決まらず、胴体が受ける法線力が細長体理論の値に届かない。
後端から二重わき出しの後流を伸ばす方式では、胴体の法線力を回復できない。
そこで底面の後ろに、半径 :math:`r_b` の円筒を ``tail_radii`` :math:`\times r_b` の長さだけ延ばし、半球で閉じた **固体の尾部フェアリング** を置いて
剥離領域の代わりとする（コサイン間隔で 8 分割の円筒と、8 分割の半球）。
フェアリングのパネルはメッシュに含めて連立方程式を解くが、力・摩擦・圧力出力の積分からは除く（``Part::Tail``）。

開口のままでは、胴体のみの :math:`C_{N\alpha}` が細長体理論の値に届かず、圧力中心が機体の前方に出るという非物理的な結果になる。
フェアリングを長くすると細長体理論の値に近づき、既定の 6 半径では、12 半径との差が小さい。
これは後端の流れを固体で置き換えた近似であり、フェアリングの長さに値が依存すること自体は、この近似の不確かさに含まれる。
``tail_radii`` を変えたときの値は :ref:`sec-vaero-panel-tail` に示している。

フィンの後流
~~~~~~~~~~~~

フィンの後縁から、後流のパネルを :math:`+x` 方向にまっすぐ伸ばす。後流は二重わき出しだけを持つ。
後縁の弦に沿った各スパン位置に 4 枚のパネルを置く。最初の長さは全長 :math:`L` の 1/4 で、
全体の長さが ``wake_length`` :math:`\times L`\ （ただし :math:`L/2` 以上）になるように長さを等比数列で増やす。
後流が長さ 2 L 以上であれば、:math:`C_{N\alpha}` はほとんど変わらない。
後流を置かない（``wake_length = 0``）と、Kutta 条件が働かず、:math:`C_{N\alpha}` は大きく外れる。
長さを変えたときの値は :ref:`sec-vaero-panel-tail` に示している。

サンプル機体の既定のメッシュは、表面 5888 枚（尾部フェアリング 768 枚を含む）、後流 160 枚である。

亜音速：Morino の定式化
------------------------

この節では、亜音速の流れを表面のわき出しと二重わき出しで表し、パネルの強さを決める連立方程式を作って解き、圧力と力を得るまでを式で追う。
流れは非圧縮で考え（:math:`M=0`）、圧縮性は最後に Göthert の変換で入れる。

定式化
~~~~~~

最初に、表面に置く二つの特異点のうち、どちらが既知でどちらが未知かを決める。壁面を流れが貫通しない条件はわき出しの強さにそのまま入り、物体の内部のポテンシャルをゼロとおく条件が二重わき出しの強さを決める方程式になる。以下でこの二つを式にする。

一様流 :math:`\bm U`\ （大きさ 1 に規格化する）中の物体のまわりの流れを、攪乱ポテンシャル :math:`\Phi` で表す。
全速度は :math:`\bm V=\bm U+\nabla\Phi` で、非圧縮ポテンシャル流では :math:`\nabla^2\Phi=0` を満たす。
物体表面 :math:`S` の外側の領域を :math:`V_e`、内側を :math:`V_i` とする（:numref:`fig-panel-morino`）。

.. _fig-panel-morino:

.. figure:: _generated/tikz/panel_morino.*
   :width: 90%

   Morino の Dirichlet 型の設定（断面の概念図）。表面 :math:`S` に二重わき出し :math:`\mu` とわき出し :math:`\sigma`、
   物体の内部ではポテンシャルをゼロとする。後縁から出る後流 :math:`W` は二重わき出しだけを持ち、強さは後縁の上面と下面の差で決まる。

Green の恒等式を外部領域に適用すると、:math:`\Phi` は表面 :math:`S` と後流 :math:`W` 上の特異点の分布で表せる :cite:`morino,katz`\ 。
物体の内部の攪乱ポテンシャルを :math:`\Phi_i=0` と選ぶ（Dirichlet 条件）と、表面上の二重わき出しの強さは外側と内側のポテンシャルの差

.. math::

   \mu = \Phi - \Phi_i = \Phi

そのものになり、わき出しの強さは表面の法線方向の流速条件から

.. math::
   :label: eq-pn-sigma

   \sigma = -\bm n\cdot\bm U

と、解くことなく決まる（:math:`\bm n` は外向き単位法線）。したがって未知数は二重わき出し :math:`\mu` だけである。

内部の点 :math:`P` でのポテンシャルは、表面の特異点と後流の二重わき出しによる寄与の和で、これがゼロになる。

.. math::
   :label: eq-pn-dirichlet

   \Phi_i(P)=\int_S\left[\mu\,\frac{1}{4\pi}\frac{\bm r\cdot\bm n}{r^3}-\sigma\,\frac{1}{4\pi r}\right]\dd S
   +\int_W\mu_w\,\frac{1}{4\pi}\frac{\bm r\cdot\bm n}{r^3}\,\dd S = 0

ここで :math:`\bm r` は積分点から :math:`P` へのベクトルである。二重わき出しの項は、パネルが点 :math:`P` に対して張る立体角 :math:`\Omega` を使って
:math:`\mu\,\Omega/4\pi` と書ける（パネルの外向き法線側から見て正）。

後流の二重わき出し :math:`\mu_w` は、後縁の **Kutta 条件** で決める。後縁を流れが滑らかに離れるためには、
後縁での圧力が上面と下面で等しくなければならない。後縁の上面側パネルの二重わき出しを :math:`\mu_u`、下面側を :math:`\mu_l` とすると、
後縁から流れ去る渦は後流に沿って保存されるので、

.. math::
   :label: eq-pn-kutta

   \mu_w = \mu_u - \mu_l

とする。:math:`\mu_w` は未知数 :math:`\mu_u,\mu_l` の線形結合なので、後流の影響は表面の列に畳み込める。

離散化
~~~~~~

表面を有限枚のパネルに分け、各パネルの強さを一定とすると、積分の条件は連立一次方程式になる。未知数はパネルの数だけの二重わき出しで、右辺は一様流から決まる既知の量である。

表面を :math:`n` 枚のパネルに分け、各パネルで :math:`\mu_j,\sigma_j` を一定とする。:eq:`eq-pn-dirichlet` の条件を各パネルの重心（内側へ向けて極限をとった点）で課すと、

.. math::
   :label: eq-pn-system

   \sum_{j}A_{ij}\,\mu_j = b_i,\qquad
   A_{ij} = \begin{cases} D_{ij} & (i\ne j)\\ -\dfrac12 & (i=j)\end{cases}
   +\bigl[\text{後流の寄与}\bigr],\qquad
   b_i = -\sum_j S_{ij}\,\bm n_j\cdot\bm U_j

となる。:math:`D_{ij}=\Omega_{ij}/4\pi` はパネル :math:`j` の単位強度の二重わき出しがパネル :math:`i` の重心につくるポテンシャル、
:math:`S_{ij}=\frac{1}{4\pi}\int_j \dd S/r` は単位強度のわき出しが作るポテンシャル（符号を除く）である。
自己項の :math:`-\tfrac12` は、平らな二重わき出しの面の内側へ近づいたときの極限値である（外側では :math:`+\tfrac12`）。
後流の寄与は、後流パネルが重心 :math:`i` に作る二重わき出しのポテンシャル :math:`W_i` を、\ :eq:`eq-pn-kutta` により
:math:`A_{i,u}\mathrel{+}=W_i`, :math:`A_{i,l}\mathrel{-}=W_i` と上面側・下面側パネルの列に加える形で入れる。

影響係数
~~~~~~~~

連立方程式の係数は、あるパネルの単位強度の特異点が、別のパネルの重心につくる影響である。平面多角形のパネルでは閉じた式で計算でき、遠くのパネルどうしには計算を省く近似を使う。

平面多角形パネルの影響係数は閉じた式で得られる :cite:`hesssmith,newman,katz`\ 。
二重わき出しは立体角で書け、多角形を三角形に分割して各三角形の立体角を

.. math::
   :label: eq-pn-solid-angle

   \Omega_\triangle = -2\arctan\frac{\bm a\cdot(\bm b\times\bm c)}{abc+(\bm a\cdot\bm b)c+(\bm a\cdot\bm c)b+(\bm b\cdot\bm c)a}

（:math:`\bm a,\bm b,\bm c` は場の点から三角形の 3 頂点へのベクトル、:math:`a,b,c` はその長さ。象限を区別する二引数の逆正接を使う :cite:`oosterom`\ ）で求めて足す。
わき出しの面積分は、パネルの面内の座標で書いた辺ごとの項

.. math::
   :label: eq-pn-source

   \int\frac{\dd S}{r} = \sum_{k}A_k\ln\frac{r_k+r_{k+1}+d_k}{r_k+r_{k+1}-d_k} - z\,\Omega,\qquad
   A_k=\frac{(y-y_k)\,\Delta x_k-(x-x_k)\,\Delta y_k}{d_k}

（:math:`d_k` は辺の長さ、:math:`r_k` は場の点から角点までの距離、:math:`(x,y,z)` は場の点のパネル座標）で与えられる :cite:`newman`\ 。
場の点がパネル径の 5 倍より遠いときは、計算を省くため点わき出し・点二重わき出しの近似

.. math::

   S\simeq\frac{A}{4\pi r},\qquad D\simeq\frac{A\,\bm r\cdot\bm n}{4\pi r^3}

に切り替える（``influence.rs`` の ``FAR_FIELD``）。厳密式と数値積分の一致、遠方近似の誤差、:math:`z\to0^\pm` での二重わき出しの値 :math:`\pm1/2` は単体テストで確かめている。

3 つの右辺と解
~~~~~~~~~~~~~~

係数行列は迎角に依存しないので、一度分解すれば、向きの違う三つの流れを同時に解ける。迎角が変わっても解き直さず、解を重ね合わせて使う。

係数行列 :math:`A` は密な :math:`n\times n` 行列で、``faer`` による部分ピボット付き LU 分解で解く。
組み立てた行列をそのまま分解する（in place）ので、:math:`n\times n` の行列は 1 つだけで、必要なメモリは :math:`8n^2+64n` バイト（既定のメッシュの表面 5888 枚で約 278 MB）である。
この見積もりは 128 ビットの整数で計算し、64 ビットの最大値で頭打ちにする（約 :math:`1.5\times10^9` 枚から）。どれほど大きなパネル数でも、桁あふれせずにメモリの上限と比べられる。
行の順に組み立てた行列を列の順に読むと :math:`A^\top` になるので、:math:`A^\top` を分解し、転置した因子で :math:`A\bm x=\bm b` を解く。
解く前に、必要な量を、このプロセスが使えるメモリの最小の値と比べる。Linux では ``/proc/meminfo`` の ``MemAvailable``、cgroup の上限（v2 の ``memory.max`` から ``memory.current`` を引いた値、v1 の ``memory.limit_in_bytes`` から ``memory.usage_in_bytes`` を引いた値）、
アドレス空間の上限 ``RLIMIT_AS``\ （``ulimit -v``）から確保済みのアドレス空間（``/proc/self/status`` の ``VmSize``）を引いた値、macOS では物理メモリ（``sysctl hw.memsize``）である。
超えるときは ``error[IY-A018]: the panel method needs about … GB for … panels (dense matrix), more than the … GB of memory available to this process (…); use coarser panel settings (body_axial, body_circ, fin_chord, fin_span)`` のエラーで止まる（括弧内は値を取った出所）。
どれも得られない環境では確かめない。行列の確保も失敗をエラーとして扱い（作業領域の余裕として行列の 1/16 と 256 MB を足して先に確かめる）、
確保できなければ ``could not allocate the dense panel matrix: about … GB for … panels; use coarser panel settings (body_axial, body_circ, fin_chord, fin_span) or free memory`` のエラーになる。
メモリの予算 ``resources.memory_gb`` を指定したときは、それも超えてはならない（:doc:`usage` の ``[resources]``）。
:math:`A` は形状と Mach 数だけで決まり、迎角によらない。右辺 :math:`b` は一様流 :math:`\bm U` の線形関数なので、
次の三つの単位問題を同時に解けば（一つの LU 分解に右辺 3 列）、任意の迎角と回転が線形結合で表せる。

.. list-table::
   :header-rows: 1
   :widths: 14 40 46

   * - 番号
     - 一様流 :math:`\bm U`\ （機体座標の :math:`x,y,z`）
     - 用途
   * - 0
     - :math:`(1,0,0)`
     - 機軸方向の流れ。基準の速度分布
   * - 1
     - :math:`(0,0,1)`
     - 横方向の流れ。迎角の線形応答
   * - 2
     - :math:`(0,0,x)`
     - ノーズ先端まわりの単位回転（位置 :math:`x` での局所的な横流れ速度が :math:`x`）

迎角 :math:`\alpha` の流れ :math:`\bm U=(\cos\alpha,0,\sin\alpha)` の解は、:math:`\cos\alpha\times(0)+\sin\alpha\times(1)` である。

表面の速度と圧力係数
~~~~~~~~~~~~~~~~~~~~

解けた二重わき出しの強さから各パネルの表面の流速を求め、Bernoulli の式で圧力係数にする。必要なのは壁面に沿う成分だけで、二重わき出しの表面での勾配から得る。

表面の速度は、パネル面内の成分が決まれば十分である（法線成分は境界条件で 0）。
全速度の面内成分は、一様流の面内成分と二重わき出しの表面勾配の和

.. math::
   :label: eq-pn-surface-velocity

   \bm V_t = \bm U-(\bm U\cdot\bm n)\bm n+\nabla_s\mu

で、:math:`\nabla_s\mu` は :math:`\mu` の面内勾配である。勾配は、各パネルについて辺で隣り合うパネルを辺のまわりに同一平面へ展開し、
重心間の変位ベクトル :math:`\bm d_m` と :math:`\mu` の差 :math:`\Delta\mu_m` から、重み :math:`1/|\bm d_m|^2` の最小二乗法

.. math::

   \min_{\bm g}\ \sum_m \frac{\bigl(\bm g\cdot\bm d_m-\Delta\mu_m\bigr)^2}{|\bm d_m|^2}

で求める（パネル面内の 2 成分の 2 × 2 の方程式）。折れ目をまたぐ隣接関係は展開によって距離を歪めない。
後縁の Kutta 線をはさむ上面と下面のパネルは、:math:`\mu` が不連続なので隣接とみなさない。

圧力係数は Bernoulli の式から（:math:`|\bm U|=1`）

.. math::
   :label: eq-pn-cp

   C_p = 1-|\bm V_t|^2

である。三つの単位問題の速度 :math:`\bm V_x,\bm V_z,\bm V_r` が分かっているので、迎角 :math:`\alpha` での表面速度は
:math:`\bm V_x\cos\alpha+\bm V_z\sin\alpha` で、:numref:`fig-panel-cp` の分布はこれから作る。
迎角についての微分は :math:`\alpha=0` で

.. math::
   :label: eq-pn-dcp

   \frac{\partial C_p}{\partial\alpha}\bigg|_{\alpha=0}=-2\,\bm V_x\cdot\bm V_z,\qquad
   \frac{\partial C_p}{\partial\omega}\bigg|_{\omega=0}=-2\,\bm V_x\cdot\bm V_r

となる（:math:`\omega` は :math:`\bm V_r` の問題の回転の大きさ。:math:`|\bm V_x|^2` は :math:`\alpha` の偶関数なので微分に寄与しない）。

.. _fig-panel-cp:

.. figure:: _generated/plots/panel_cp.*
   :width: 100%

   サンプル機体の表面圧力係数（:math:`M=0`、:math:`\alpha=4^\circ`）。上: 胴体を周方向に展開した図。ノーズ先端のよどみ点（赤）、肩の吸い込み（青）が見える。
   フィンの付け根の範囲（:math:`x\gtrsim1.3` m）では、フィンとの干渉で周方向に圧力が変わる。
   下: 1 枚のフィンの両面（パネルごとの値をそのまま塗った図）。風上側（右）は風下側（左）より圧力が高い（前縁付近は圧縮）。風下側の前縁で吸い込みが強い。

力とモーメント
~~~~~~~~~~~~~~

圧力係数にパネルの面積と向きを掛けて足し合わせると、力とモーメントが得られる。ここでは、迎角と回転についての微係数を、解いた解から直接作る。

圧力が面に及ぼす力は :math:`-p\bm n\,\dd S` なので、:math:`z` 方向（法線力の正の向き）の力係数への寄与は、
パネル :math:`i` について :math:`f_i=-n_{z,i}A_i/S` を使って :math:`-C_{p,i}\,n_{z,i}A_i/S=C_{p,i}\,f_i` である。
:eq:`eq-pn-dcp` を代入し、尾部フェアリングを除く全パネルで足し合わせて、次の四つの量を作る。

.. math::
   :label: eq-pn-derivs

   \begin{aligned}
   C_{N\alpha}&=\sum_i\left(\frac{\partial C_p}{\partial\alpha}\right)_i f_i,&
   M_\alpha&=\sum_i\left(\frac{\partial C_p}{\partial\alpha}\right)_i \bigl(x_if_i-z_ig_i\bigr),\\
   N_{r}&=\sum_i\left(\frac{\partial C_p}{\partial\omega}\right)_i f_i,&
   M_{r}&=\sum_i\left(\frac{\partial C_p}{\partial\omega}\right)_i \bigl(x_if_i-z_ig_i\bigr)
   \end{aligned}

:math:`(x_i,z_i)` はパネルの重心の位置、:math:`g_i=-n_{x,i}A_i/S` は単位の :math:`C_p` あたりの軸方向の力で、:math:`M_\alpha` と :math:`M_r` はノーズ先端まわりのモーメントである。
モーメントは :math:`\bm r\times\bm F` のピッチ成分 :math:`x\,F_z-z\,F_x` をすべて取り（符号は :math:`x_{cp}=M/C_N` の約束）、:math:`x\,F_z` だけではない。
ノーズ、ボートテール、フィンの縁のような傾いた面では、圧力の軸方向の力が横方向の腕 :math:`z_i` を持つからである。
球ではすべての圧力の力が中心を通るので、ノーズ先端まわりのモーメントは中心の腕と法線力の積になる。これを単体テスト（``subsonic_nose_moment_is_the_full_cross_product``）で確かめている。
:math:`\alpha\to0` の圧力中心は :math:`x_{cp}=M_\alpha/C_{N\alpha}` である。

圧縮性：Göthert の変換
~~~~~~~~~~~~~~~~~~~~~~

ここまでは非圧縮の流れである。亜音速の圧縮性は、機体を横方向に縮めた物体で非圧縮の問題を解く Göthert の変換で入れる。
なお、この節の :math:`\beta=\sqrt{1-M^2}` は圧縮性の因子であり、機体座標の横滑り角 :math:`\beta`\ （:doc:`aero_coefficients`）とは別の量である（式のある場所ごとに断る。衝撃波角は :math:`\beta_s` と書く）。

亜音速の圧縮性流れの線形化方程式 :math:`(1-M^2)\Phi_{xx}+\Phi_{yy}+\Phi_{zz}=0` は、
:math:`\beta=\sqrt{1-M^2}` として横方向の座標を :math:`y'=\beta y`, :math:`z'=\beta z` と縮めると Laplace 方程式になる。
物体の表面での流れの接線条件を、縮めた物体で非圧縮の問題とみなすと（軸対称の場合 :math:`\Phi=\Phi'/\beta^2` として確かめられる）、
Göthert の規則 :cite:`gothert` が得られる。

.. math::
   :label: eq-pn-gothert

   C_p(M,\alpha) = \frac{1}{\beta^2}\,C_p'(\alpha'),\qquad \alpha'=\beta\,\alpha

:math:`C_p'` は、:math:`y,z` を :math:`\beta` 倍に縮めた物体の非圧縮解である。
Prandtl–Glauert の規則 :cite:`glauert` を三次元の物体に拡張したもので、薄い物体で、局所の流れがほぼ亜音速である範囲で成り立つ。
実装では、メッシュの :math:`y,z` 座標を :math:`\beta` 倍にして非圧縮の解を求め、
その速度分布を **元の形状** のパネル（面積・法線・重心）に割り当てて積分する。このとき\ :eq:`eq-pn-dcp` の迎角微分は
:math:`\partial C_p/\partial\alpha=\beta\,(-2\bm V_x\cdot\bm V_z)/\beta^2=-2\bm V_x\cdot\bm V_z/\beta` となり、\ :eq:`eq-pn-derivs` にはこの値を使う。
:math:`M=0` では :math:`\beta=1` で元の非圧縮の解に一致する。

``subsonic_machs`` の各 Mach 数（既定は 0, 0.3, 0.5, 0.6, 0.7, 0.8 の 6 点）で上の解を一度ずつ求め、
その間の Mach 数での :math:`C_{N\alpha}, M_\alpha, N_r, M_r` は、Fritsch–Carlson の単調 3 次補間（pchip :cite:`fritsch`）で内挿する。
最も低い解の Mach 数より低い範囲は端の値で一定とする。

ピッチ減衰の和
~~~~~~~~~~~~~~

ピッチ減衰（:ref:`sec-aero-damping`）に必要な三つの和を、パネルの解から作る。部品積み上げ法の和と同じ意味で、重心の位置によらず厳密に成り立つように定義している。

:doc:`barrowman` の\ :eq:`eq-damping` の :math:`S_0,S_1,S_2` を、四つの量から次のように作る。

.. math::
   :label: eq-pn-damping

   S_0=C_{N\alpha},\qquad S_1=\frac{M_\alpha+N_r}{2},\qquad S_2=M_r

部品積み上げ法では :math:`M_\alpha=N_r=S_1`、:math:`M_r=S_2` が恒等的に成り立つ。パネル法ではこれらは一般には少しずれる。
重心まわりのピッチ角速度 :math:`\omega` の運動は、ノーズ先端まわりの回転 :math:`\omega` と、一様な迎角 :math:`-\omega x_{cg}/V` の重ね合わせである。
したがって法線力は :math:`(N_r-x_{cg}S_0)\omega/V`、ノーズまわりのモーメントは :math:`(M_r-x_{cg}M_\alpha)\omega/V` で、重心まわりのモーメントは

.. math::

   M_{cg}=M_{\mathrm{nose}}-x_{cg}N=\frac{\omega}{V}\Bigl[M_r-x_{cg}\,(M_\alpha+N_r)+x_{cg}^2S_0\Bigr]

となる。この角括弧の中が、\ :eq:`eq-damping` の :math:`S_2-2x_{cg}S_1+x_{cg}^2S_0` と一致するのは、:math:`S_1=(M_\alpha+N_r)/2` とおいたときである。
つまり :math:`S_1` の定義は任意ではなく、重心位置によらず厳密に成り立つように選んでいる。

非線形な法線力
~~~~~~~~~~~~~~

パネルの解は迎角について線形だが、係数表は迎角 90°（既定の ``alpha_max_deg``）まである。迎角が大きいときに胴体を横切る流れがはく離して生じる横流れ力を、部品積み上げ法と同じ経験式で加える。

係数表は大きな迎角まであるので、線形の応答だけでは足りない。亜音速の列では、ポテンシャル流の寄与を:doc:`barrowman` と同じ形で

.. math::
   :label: eq-pn-cn-sub

   C_N = C_{N\alpha}\sin\alpha\cos\alpha + C_N^{\mathrm{cross}},\qquad
   C_N^{\mathrm{cross}} = K\,\frac{A_p}{S}\sin^2\alpha\quad(K=1.1)

とし、モーメントは :math:`M_\alpha\sin\alpha\cos\alpha+C_N^{\mathrm{cross}}\,x_p`\ （:math:`x_p` は側面投影面積の図心）とする。
第 2 項は、迎角が大きいときに胴体を横切る流れがはく離して生じる粘性の横流れ力 :cite:`allen` を、:doc:`barrowman` の\ :eq:`eq-body-lift` と同じ経験式で与えたものである。
圧力中心は :math:`x_{cp}=M/C_N`、:math:`\alpha=0` では :math:`M_\alpha/C_{N\alpha}` とする。

この式は付着した流れの線形理論なので、小さい迎角でしか成り立たない。ポテンシャル流の項 :math:`\sin\alpha\cos\alpha` は :math:`90^\circ` で 0 になるので、
そのまま使うと大きな迎角でフィンが法線力を持たず、圧力中心は胴体の側面投影面積の図心まで前へ動く。
実際には、大きな迎角のフィンは流れに垂直な平板に近く、横流れの力を受ける（:ref:`sec-fin-large-alpha`）。
そこで亜音速の列は、付着流の角度 :math:`\alpha_1=15^\circ`\ （部品積み上げ法のフィンの式と同じ角度）までは上の式のまま使い、それより大きい迎角では部品積み上げ法の値 :math:`B(\alpha)`\ （胴体、胴体の横流れ、平面形の図心に働くフィンの平板の横流れ。:eq:`eq-cn-total`、:eq:`eq-xcp-total`）へ移す。
法線力 :math:`C_N` とノーズ先端まわりのモーメント :math:`C_Nx_{cp}` のそれぞれ（:math:`P` と書く）について、

.. math::
   :label: eq-pn-large-alpha

   P(\alpha)=B(\alpha)+\bigl(D+D'(\alpha-\alpha_1)\bigr)\,w(\alpha),\qquad
   D=P_{\mathrm{panel}}(\alpha_1)-B(\alpha_1),\quad D'=P_{\mathrm{panel}}'(\alpha_1)-B'(\alpha_1)

とする。:math:`P_{\mathrm{panel}}` は上のパネル法の式、:math:`w` は :math:`\alpha_1` で 1、:math:`2\alpha_1=30^\circ` で 0 になる、両端の傾きが 0 の 3 次 Hermite 補間（C\ :sup:`1` のなめらかな階段）である。
値と傾きは :math:`\alpha_1` でパネル法の値に一致し、:math:`30^\circ` から先は部品積み上げ法の値 :math:`B(\alpha)` そのものになる。
遷移の間の圧力中心 :math:`x_{cp}=M/C_N` は、機体の内側（0 から全長）に制限する。:math:`\alpha_1` までの値は変えないので、:math:`C_{N\alpha}`、迎角 0 の圧力中心、減衰の和は変わらない。
超音速の列（局所傾斜法）は、各迎角の力を直接積分するので、この移行を使わない。
単体テスト（``subsonic_large_angles_carry_the_fin_crossflow``）で、:math:`\alpha_1` まではポテンシャル流と横流れの式に、:math:`30^\circ` 以上（:math:`110^\circ` まで）では部品積み上げ法の値に一致し、遷移の中で :math:`C_N` が連続で単調に増えることと、:math:`90^\circ` の圧力中心がフィンの横流れで胴体の図心より 0.05 m 以上後ろにあることを確かめている。

フィンと胴体の干渉
~~~~~~~~~~~~~~~~~~

パネル法ではフィンと胴体の干渉が形状から自然に入る。参考のため、細長体理論による見積りと比べる。
円形の胴体（半径 :math:`r`）に、先端半径 :math:`s_t=r+s`\ （:math:`s` は露出したスパン）の平板フィンがついた翼胴を、細長体の横断面の問題として考える。
等角写像 :math:`\zeta=z+r^2/z` で円とフィンを一枚の平板に写すと、横流れの見かけの質量が求まる。
露出フィンだけが生む揚力（フィンの根元を胴体が覆っていない単独のフィン）に対する、翼胴全体の揚力の増加の比は

.. math::
   :label: eq-pn-slender-k

   K_{W(B)}+K_{B(W)}=\frac{(1-\tau^2)^2}{(1-\tau)^2}=(1+\tau)^2,\qquad \tau=\frac{r}{s_t}

である :cite:`pnk`\ （Pitts, Nielsen, Kaattari の細長体の結果であり、ここでは上の簡単な写像で再導出して確かめた）。
:math:`K_{W(B)}` は胴体のあるときのフィンの揚力、:math:`K_{B(W)}` はフィンの存在による胴体への揚力の持ち上げである。
サンプル機体では :math:`r=0.05` m、:math:`s_t=0.15` m で :math:`\tau=1/3`、:math:`(1+\tau)^2=1.78` になる。
一方、部品積み上げ法の係数は :math:`K_{fb}=1+r/(s+r)=1.33` である（:eq:`eq-fin-total`）。

超音速：局所傾斜法
-------------------

超音速では、各パネルは自分の傾きだけに反応して圧力が決まるので、連立方程式を解かずに圧力係数を求められる。この節では、傾きから圧力を作る関係（くさび、膨張、円すい）と、それを機体の部品ごとにどう使い分けるかを述べる。

超音速の流れでは、ある点の圧力はその点の上流側の形状だけで決まり、各パネルが自分の傾き（流れに対する面の傾斜角）にほぼ局所的に反応する。
局所傾斜法はこの性質を利用して、各パネルの圧力係数を、流れに対するパネルの **傾斜角** :math:`\delta` だけから決める。
流れの向きの単位ベクトル :math:`\bm u=(\cos\alpha,0,\sin\alpha+\omega x)/|\cdot|`\ （回転 :math:`\omega` のときは位置 :math:`x` での局所流速を含む。フィンのパネルでは横流れの成分を胴体の吹き上げ :eq:`eq-pn-upwash` に置き換える）と
外向き法線 :math:`\bm n` から

.. math::

   \delta=\arcsin(-\bm n\cdot\bm u)

とし、:math:`\delta>0` を圧縮（風上）、:math:`\delta<0` を膨張（風下）とする。

接線くさびと膨張
~~~~~~~~~~~~~~~~

流れに向かって傾いた面（風上）には斜め衝撃波、流れに背を向けた面（風下）には膨張波がつき、圧力が変わる。ここでは、傾斜角から圧力係数を作る二つの関係を示す。
なお、衝撃波角には :math:`\beta_s` を使う。機体座標の横滑り角 :math:`\beta` や、亜音速の圧縮性の因子 :math:`\sqrt{1-M^2}` とは別の量である。

圧縮のとき、傾斜角 :math:`\delta` のくさびに付く斜め衝撃波の角度 :math:`\beta_s` は :math:`\theta`–:math:`\beta`–:math:`M` の関係 :cite:`naca1135,liepmann`

.. math::
   :label: eq-pn-tbm

   \tan\delta = \frac{2\cot\beta_s\,(M^2\sin^2\beta_s-1)}{M^2(\gamma+\cos2\beta_s)+2}

の弱い解（Mach 角から最大偏向角に対応する角の間を二分法で解く）で、圧力係数は

.. math::
   :label: eq-pn-wedge

   C_p=\frac{4}{\gamma+1}\left(\sin^2\beta_s-\frac{1}{M^2}\right)

である（:math:`\gamma=1.4`）。
:math:`\delta` が最大偏向角 :math:`\delta_d` を超えて衝撃波が離脱するときは、修正 Newton 則 :cite:`anderson` の :math:`\sin^2\delta` の形を、離脱の角度で付いた衝撃波の値から始まるように縮めた

.. math::
   :label: eq-pn-detached

   C_p=C_{p,d}+\bigl(C_{p,\max}-C_{p,d}\bigr)\,\frac{\sin^2\delta-\sin^2\delta_d}{\cos^2\delta_d}\qquad(\delta_d<\delta\le90^\circ)

を使う。:math:`C_{p,d}` は :math:`\delta_d` での付いた衝撃波の圧力係数（:eq:`eq-pn-wedge` を最大偏向角に対応する衝撃波角で評価した値）、
:math:`C_{p,\max}` は垂直衝撃波の後ろのよどみ点圧力の係数である。
この式は :math:`\delta=\delta_d` で :math:`C_{p,d}` に、:math:`90^\circ` で :math:`C_{p,\max}` になり、圧力係数は傾斜角についても Mach 数についても離脱の前後で連続である。
修正 Newton 則 :math:`C_{p,\max}\sin^2\delta` をそのまま使うと、離脱の角度で圧力係数が下へ跳ぶ（:math:`M=2` のくさびでは、離脱の角度 22.97° で 0.945 から 0.25 へ）。
連続であることは、:math:`M=1.05` から 5 で 0.01° 刻みに傾斜角を走査し、離脱の前後の差が :math:`10^{-3}` 未満で、:math:`C_p` が傾斜角について減らず、:math:`90^\circ` で :math:`C_{p,\max}` になることで確かめている（``fin_cp_is_continuous_across_shock_detachment``）。
:math:`M\le1` では修正 Newton 則 :math:`C_p=C_{p,\max}\sin^2\delta` を使う。

膨張（:math:`\delta<0`）のとき、Mach 数 :math:`M` の流れが角度 :math:`|\delta|` だけ膨張する Prandtl–Meyer 膨張 :cite:`liepmann,naca1135` を使う。
Prandtl–Meyer 関数

.. math::

   \nu(M)=\sqrt{\frac{\gamma+1}{\gamma-1}}\arctan\sqrt{\frac{\gamma-1}{\gamma+1}(M^2-1)}-\arctan\sqrt{M^2-1}

について :math:`\nu(M_2)=\nu(M)+|\delta|` を解いて膨張後の Mach 数 :math:`M_2` を求め、等エントロピーの関係

.. math::

   C_p=\frac{(p_2/p)-1}{\tfrac12\gamma M^2},\qquad
   \frac{p_2}{p}=\left(\frac{1+\tfrac{\gamma-1}{2}M^2}{1+\tfrac{\gamma-1}{2}M_2^2}\right)^{\gamma/(\gamma-1)}

で圧力係数を得る。膨張が真空の限界（:math:`\nu_{\max}`）に達したら :math:`C_p=-2/(\gamma M^2)` で止める。

Taylor–Maccoll の接線円すい
~~~~~~~~~~~~~~~~~~~~~~~~~~~

回転体のノーズや肩の面には、くさびの代わりに、同じ傾斜角の円すいの表面圧力を使う（接線円すい）。
円すいのまわりの流れは Taylor–Maccoll の方程式を数値積分して求め、結果はあらかじめ表にしておく。

頂角 :math:`\delta` の円すいが超音速の流れに置かれたとき、円すいの頂点を中心とする球座標 :math:`(r,\theta)` で、
速度を最大速度で無次元化した成分 :math:`(V_r,V_\theta=\dd V_r/\dd\theta)` は Taylor–Maccoll の方程式 :cite:`taylormaccoll`

.. math::
   :label: eq-pn-tm

   \frac{\dd^2V_r}{\dd\theta^2}\Bigl(a^2-V_\theta^2\Bigr)=V_\theta^2V_r-a^2\left(2V_r+V_\theta\cot\theta\right),\qquad
   a^2=\frac{\gamma-1}{2}\left(1-V_r^2-V_\theta^2\right)

に従う。衝撃波の角度 :math:`\beta_s` を与えると、衝撃波の直後の速度成分が斜め衝撃波の関係から決まり、
これを初期値として :math:`\theta` を :math:`\beta_s` から減らす向きに 4 次の Runge–Kutta 法で積分し（シューティング）、
:math:`V_\theta=0` になる角度が円すいの表面、すなわち円すいの半頂角 :math:`\delta_c` である。
表面の圧力は、衝撃波直後の圧力に等エントロピーの比 :math:`\bigl[(1-V_r^2)/(1-V_2^2)\bigr]^{\gamma/(\gamma-1)}` を掛けて得る。

この積分を飛翔計算の中で毎回行うと遅いので、あらかじめ表にする。Mach 数 :math:`1.02` から :math:`0.04` 刻みで 150 点、
各 Mach 数で衝撃波角を Mach 角に近い側に細かく（半径方向に等比数列で）600 点とり、円すい半角 0.25° 刻みの圧力係数に整理する。
表の範囲内は線形に内挿し、最初の区間の下側は :math:`C_p\propto\delta^2` とする。
表の各 Mach 数の行は、その行の最後の角度 :math:`\delta_d`\ （離脱限界）までの値を持つ。
:math:`\delta_d` を超える角度では、その行の最後の値を :math:`C_{p,d}` として :eq:`eq-pn-detached` で :math:`C_{p,\max}` まで上げ、そのうえで隣り合う行を Mach 数について内挿する。
したがって、円すいの圧力係数も、角度と Mach 数について離脱の前後で連続である（``cone_cp_is_continuous_across_shock_detachment``）。
表の Mach 数の範囲より下（:math:`M\le1.0`）では、修正 Newton 則 :math:`C_{p,\max}\sin^2\delta` を使う。

回転体の傾斜と迎角の増分
~~~~~~~~~~~~~~~~~~~~~~~~

胴体のパネルは、迎角がないときの傾きでの圧力を基準にして、迎角による傾きの変化を線形に足す。この足し方の傾きの選び方が、細長体理論との一致を決める。

胴体のパネルでは、迎角がないときの傾斜（軸対称の傾斜角）\ :math:`\delta_0=\arcsin(-n_x)` と、実際の傾斜角 :math:`\delta` を区別する。
圧力係数は

.. math::
   :label: eq-pn-body-cp

   C_p = C_{p,0}(\delta_0) + 4\sin\delta_0\cos\delta_0\,(\delta-\delta_0)

とし、結果を :math:`[-2/(\gamma M^2),\,C_{p,\max}]` に制限する。:math:`C_{p,0}` は :math:`\delta_0>0` なら接線円すい（離脱後は :eq:`eq-pn-detached`）、
:math:`\delta_0<0`\ （ボートテールなど）なら Prandtl–Meyer 膨張、:math:`\delta_0=0`\ （円筒）なら 0 である。
右辺の第 2 項は迎角（や回転）による傾斜角の変化を、傾き :math:`4\sin\delta_0\cos\delta_0` で線形に足すものである。

この傾きは、細長体の圧力法則 :math:`C_p=2\delta^2` の微分 :math:`4\delta` に対応する。
円すいの表面圧力 :math:`C_{p,\mathrm{cone}}(\delta_0)` そのものの傾き :math:`2C_{p,\mathrm{cone}}/\delta_0`\ （:math:`C_p\propto\delta^2` ではなく円すいの対数則に従うと仮定）を使うと、
半頂角 10° の円すい-円筒の :math:`M=2` での :math:`C_{N\alpha}` が約 3.4 になり、細長体理論の 2 を大きく上回る。
:math:`4\sin\delta_0\cos\delta_0` を使うと細長体の値 :math:`2\,\dd S/\dd x` に一致する（値は :ref:`sec-vaero-panel` にある）。
円筒では :math:`\delta_0=0` で傾きも 0 なので、増分が一次の法線力を生まない。
局所の二次元的な膨張を円筒の風下側にそのまま使うと、一次の法線力が見かけ上生じてしまうからである。
円筒の二次の横流れ力は、前述の\ :eq:`eq-pn-cn-sub` の経験項（:math:`C_N^{\mathrm{cross}}`）で与える。

フィンの傾斜と翼端の Mach 円すい
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

フィンも傾斜角からくさびと膨張で圧力を決めるが、厚みによる傾きを基準にし、翼端の三次元効果を粗く補正する。

フィンのパネルは、傾斜角 :math:`\delta>0` なら接線くさび（離脱後は :eq:`eq-pn-detached`）、:math:`\delta<0` なら Prandtl–Meyer 膨張とする。
フィンは迎角 0 でも厚みのために傾斜 :math:`\delta_0`\ （前縁側の面は圧縮、後縁側は膨張）を持つので、
迎角による変化分は :math:`C_p(\delta)-C_p(\delta_0)` で、

.. math::

   C_p = C_p(\delta_0) + f_{\mathrm{tip}}\,\bigl[C_p(\delta)-C_p(\delta_0)\bigr]

とする。:math:`f_{\mathrm{tip}}` は翼端の三次元効果である。二次元の局所傾斜法は、翼端から広がる Mach 円すいの内側で圧力差が実際より大きい。
翼端前縁の角から出る Mach 円すいの内側（点 :math:`(x,\rho)` が :math:`x-x_{tip}\ge\sqrt{M^2-1}\,(r_{tip}-\rho)` を満たす領域）では迎角による増分を半分（:math:`f_{\mathrm{tip}}=1/2`）とし、
その外側では :math:`f_{\mathrm{tip}}=1` とする。:math:`x_{tip}` は翼端の前縁の位置、:math:`r_{tip}` は翼端の半径である。
これは線形理論で、円すいの内側の矩形翼の荷重が二次元の半分程度になることを簡単に取り入れたもので、精度は粗い。

フィンに当たる胴体の吹き上げ
~~~~~~~~~~~~~~~~~~~~~~~~~~~~

局所傾斜法は各パネルの傾きだけを見るので、そのままではフィンと胴体の干渉が入らない（亜音速の解は胴体のパネルを通して干渉を含む）。
そこで、フィンのパネルの傾斜角を求める流れの向きに、胴体のまわりの横流れを入れる。
迎角と回転による横流れ :math:`w=\sin\alpha+\omega x` を、胴体（フィンの位置の半径 :math:`r`）を過ぎる二次元のポテンシャル流の、パネルの位置 :math:`(y,z)` での速度に置き換える。
複素座標 :math:`\zeta=y+\mathrm{i}z` と一様な横流れ :math:`(w_y,w_z)` から

.. math::
   :label: eq-pn-upwash

   u_y-\mathrm{i}\,u_z=(w_y-\mathrm{i}\,w_z)-(w_y+\mathrm{i}\,w_z)\,\frac{r^2}{\zeta^2}

で、流れの向きは :math:`\bm u=(\cos\alpha,u_y,u_z)/|\cdot|` とする。
横流れに垂直なフィン（ピッチではフィンが水平面にあるとき）は横流れが :math:`1+r^2/\rho^2` 倍になり（胴体の吹き上げ）、
横流れの向きに並ぶフィンは、フィンに垂直な成分を持たない。
矩形のフィンでは、この倍率のスパン方向の平均は :math:`1+r/(s+r)` で、部品積み上げ法の :math:`K_{fb}`\ （:eq:`eq-fin-total`）に一致する。

吹き上げは、局所傾斜法と翼端の補正が線形理論を再現する Mach 数でだけ掛ける。
翼端の前縁の角から根元の後縁までの軸方向の距離を :math:`L`、露出したスパンを :math:`s` として :math:`q=\sqrt{M^2-1}\,s/L` とし、
吹き上げの増分（:eq:`eq-pn-upwash` の速度と一様な横流れの差）に重み

.. math::
   :label: eq-pn-upwash-weight

   k(q)=\begin{cases}0 & q\le\tfrac12\\ 3t^2-2t^3,\ t=2q-1 & \tfrac12<q<1\\ 1 & q\ge1\end{cases}

を掛ける。:math:`q\ge1` では翼端の Mach 円すいが翼弦の中で根元に届かず、根元側は二次元の流れ、翼端側は一つの自由な翼端の円すい流れで、局所傾斜法と翼端の補正（円すいの内側の荷重を半分）が線形理論に一致する。
:math:`q\le1/2` では、円すいが根元に届き、根元での反射（線形理論の反射面の像の翼端）が後縁より前で翼端に戻るので、フィン全体が翼端どうしの干渉の領域に入る。
矩形のフィンの翼端損失 :math:`1-1/(2\beta A)` も :math:`\beta A\ge1`\ （:math:`q\ge1/2`）でしか成り立たず、局所傾斜法の荷重（:math:`1/\beta` で大きくなる）には根拠がないので、吹き上げを掛けない。
その間は値と傾きの連続な 3 次式でつなぐ。データに合わせた係数ではない。
サンプル機体（:math:`L=0.12` m、:math:`s=0.1` m）では :math:`M=1.2` で :math:`k=0.03`、:math:`M=1.5` で 0.96、:math:`M\ge1.56` で 1、
Basic Finner（:math:`L=s=c`）では :math:`M=1.08` で 0、:math:`M=1.29` で 0.70、:math:`M\ge1.41` で 1 である（``upwash_is_faded_out_where_the_tip_cone_covers_the_fin``）。
フィンから胴体への持ち上げ（:math:`K_{B(W)}`）は、胴体のパネルの増分が円筒で 0 なので入らない（後部の胴体がない超音速では小さい。:math:`r/(s+r)=1/3` の矩形フィンの Mach 2 で約 0.1。細長体理論では 0.49）。
単体テスト（``supersonic_fins_see_the_body_upwash``）で、Basic Finner と同じ比率（:math:`r=s=c`）の矩形フィンの Mach 2 で、吹き上げのあるときとないときのフィンの法線力の比が、翼端の補正で重みを付けた :math:`1+r^2/y^2` の平面形の平均（1.364）と 3 % 以内で一致し（1.355）、
フィンの :math:`C_{N\alpha}` が部品積み上げ法と 6 % 以内で一致する（6.97 と 6.71）ことを確かめている。

力と微係数
~~~~~~~~~~

各パネルの力を足して、法線力・軸方向の圧力力・ノーズまわりのモーメントを求める（尾部フェアリングを除く）。
この節の :math:`C_x,C_z,M_N` はパネル計算の内部量である。:math:`x` を後方へ測る座標で、:math:`C_x` は後ろ向き（抗力の向き）を正とし、
機体座標の :math:`C_X,C_Z,C_m`\ （:doc:`aero_coefficients`）とは別に定義する。

.. math::

   C_{x}=-\sum_i C_{p,i}\,n_{x,i}\frac{A_i}{S},\qquad C_{z}=-\sum_i C_{p,i}\,n_{z,i}\frac{A_i}{S},\qquad M_N=-\sum_i C_{p,i}\bigl(n_{z,i}x_i-n_{x,i}z_i\bigr)\frac{A_i}{S}

モーメントは亜音速と同じく :math:`\bm r\times\bm F` のピッチ成分 :math:`x\,F_z-z\,F_x` である。球の圧力中心が中心に来ること（平らなパネルの法線が中心からわずかにずれる分の 3 % 以内）を単体テスト（``supersonic_nose_moment_is_the_full_cross_product``）で確かめている。

迎角 0 での :math:`C_x` が造波抗力（圧力による軸力）で、これがそのまま軸力係数の一部になる。
:math:`C_{N\alpha}`, :math:`M_\alpha`, :math:`N_r`, :math:`M_r` は、迎角と回転についての :math:`\pm0.25^\circ` の中心差分で求める
（回転は :math:`\omega=0.25^\circ/L`）。係数表の :math:`C_N` は、各迎角で局所傾斜法の力 :math:`C_z` に、\ :eq:`eq-pn-cn-sub` の横流れ項を加える。

遷音速の補間
------------

亜音速の解も局所傾斜法も成り立たない Mach 数の帯では、両側の値を滑らかにつなぐ。この帯の物理は計算しない。

Mach 数が :math:`M_{lo}=\min(` ``transonic[0]``\ :math:`,\ ` 最後の亜音速解の Mach 数 :math:`)`\ （既定は 0.8）と超音速側の下限 :math:`M_{hi}=` ``transonic[1]``\ （既定は 1.2）の間では、
どちらの手法も成り立たない。:doc:`barrowman` と同じ形の 3 次 Hermite 補間（:eq:`eq-hermite`）で両端をつなぐ。
:math:`M_{lo}` までは亜音速の解（解いた Mach 数の間の内挿）で、:math:`M_{lo}` が最後の亜音速解より高くても、亜音速の列をその解の先へ外挿しない。
``transonic[0]`` を下げると帯が広がり、``transonic[0]`` より下の列は変わらない（``transonic_band_starts_at_its_lower_edge``）。
補間する量は、流れの解について線形な量である。迎角ごとの法線力 :math:`C_N` と、そのノーズ先端まわりのモーメント :math:`C_Nx_{cp}`、軸力 :math:`C_A`\ （燃焼中・燃焼後）、
列ごとの :math:`C_{N\alpha}` とそのモーメント :math:`C_{N\alpha}x_{cp}(0)`\ （:math:`x_{cp}(0)` は迎角 0 の圧力中心）、減衰の和 :math:`S_0,S_1,S_2` である。
圧力中心は補間せず、補間したモーメントを補間した法線力で割って求める（:math:`C_N=0` の迎角 0 では、補間した :math:`C_{N\alpha}x_{cp}(0)` を :math:`C_{N\alpha}` で割る）。
比である圧力中心を直接補間すると、力・モーメント・傾きの組が互いに合わなくなるからである。
端点の値は、:math:`M_{lo}` では亜音速の列、:math:`M_{hi}` では超音速の列で、端点での Mach 数についての傾き :math:`f_0',f_1'` は、
:math:`h=0.02` だけ外側の点との片側差分で求める。補間区間に Mach 数の格子点がなければ計算しない。

軸力以外の量では、この傾きを Fritsch–Carlson の方法 :cite:`fritsch` で制限し、補間が両端の値の間に収まる（行き過ぎない）ようにする。
帯の幅を :math:`\Delta M=M_{hi}-M_{lo}`、平均の傾きを :math:`s=(f_1-f_0)/\Delta M` として、比 :math:`a=f_0'/s`、:math:`b=f_1'/s` の負のものを 0 にし（:math:`s` と逆向きの傾きを捨てる）、
:math:`r=\sqrt{a^2+b^2}>3` なら両方を :math:`3/r` 倍に縮めて、:math:`f_0'=as`、:math:`f_1'=bs` とする。:math:`f_0=f_1` なら両方の傾きを 0 とする。
超音速側の端の片側差分は、低い超音速で局所傾斜法の値が Mach 数とともに急に変わるために大きく、制限しないと帯の中で行き過ぎる。
軸力は、片側差分の傾きをそのまま使う（遷音速の抗力の立ち上がりの形を保つため）。
ただし補間は中身のない数値的な橋渡しであり、遷音速の物理（衝撃波の発生、:math:`C_{N\alpha}` のピーク位置など）を計算するものではない。

軸力
----

軸力は、表面摩擦、圧力による抗力、部品積み上げ法の経験項を足したものを、迎角による倍率で補正して表に入れる。

軸力係数は、迎角 0 の値に、:doc:`barrowman` と同じ迎角による倍率 :math:`f(\alpha)`\ （:eq:`eq-ca-alpha`）を掛けて表に入れる（``axial_alpha_factor``）。
零揚力の軸力は、次の項の和である。

.. math::
   :label: eq-pn-ca

   C_{A0}=
   \begin{cases}
   C_{f}^{\mathrm{panel}}+C_{D,\mathrm{nose}}+C_{D,\mathrm{base}}+C_{D,\mathrm{fin}}+C_{D,\mathrm{bt}}+C_{D,\mathrm{extra}} & (M\le M_{lo})\\[2pt]
   C_{f}^{\mathrm{panel}}+C_{x}^{\mathrm{wave}}+C_{D,\mathrm{base}}+C_{D,\mathrm{fin}}+C_{D,\mathrm{bt}}+C_{D,\mathrm{extra}} & (M\ge M_{hi})
   \end{cases}

:math:`C_{D,\mathrm{nose}}, C_{D,\mathrm{base}}, C_{D,\mathrm{fin}}, C_{D,\mathrm{bt}}, C_{D,\mathrm{extra}}` は:doc:`barrowman` の部品積み上げ法の項（:eq:`eq-cd0`）をそのまま使う。
:math:`C_{D,\mathrm{base}}` は燃焼中にノズル出口の面積を差し引くので、係数表に燃焼中・燃焼後の 2 種類の値がある点も同じである。
超音速では、ノーズの項の代わりに局所傾斜法で積分した造波抗力 :math:`C_x^{\mathrm{wave}}`\ （ノーズ・肩・ボートテール・フィンの厚みの寄与を含む）を使う。
遷音速は前節の補間で両端をつなぐ。

フィンの縁の項 :math:`C_{D,\mathrm{fin}}`\ （丸い前縁と角張った後縁の抗力）は、実機の鈍い縁の抗力を表す経験項であり、
パネルモデルのフィンは尖った両凸断面なので、その造波抗力（サンプルで :math:`M=2` に約 0.017）とは別の寄与である。
ただし厚みによる抗力の一部は両方に含まれ、わずかに重複する可能性がある。

表面摩擦
~~~~~~~~

各パネルの表面摩擦を、平板の局所摩擦係数から求めて足し合わせる。圧縮性は参照温度法で入れる。

表面摩擦は、パネルごとに平板の局所摩擦係数を求めて足し合わせる。走行長 :math:`x_s` は、胴体ではノーズ先端から子午線に沿った長さ、
フィンでは前縁からの弦方向の長さ、Reynolds 数は :math:`Re_x=V x_s/\nu`\ （設定した大気 ``[atmosphere]``\ （温度差を含む）の射点高度の値で :math:`V=\max(M,0.05)\,a`）である。部品積み上げ法の摩擦と同じ大気を使う。
圧縮性は Eckert の参照温度法 :cite:`eckert` で入れる。断熱壁の温度比と参照温度比は

.. math::
   :label: eq-pn-eckert

   \frac{T_w}{T}=1+r\frac{\gamma-1}{2}M^2,\qquad
   \frac{T^*}{T}=1+0.032M^2+0.58\left(\frac{T_w}{T}-1\right),\qquad r=0.89

で、参照状態での Reynolds 数 :math:`Re^*=Re_x\,(T^*/T)^{-1.76}`\ （粘性のべき乗則）から、
:math:`Re^*<5\times10^5` の層流域では Blasius の式、それ以上の乱流域では平板の式を使う :cite:`schlichting`。

.. math::
   :label: eq-pn-cf

   c_f=\begin{cases}
   \dfrac{0.664}{\sqrt{Re^*}}\dfrac{1}{T^*/T} & (Re^*<5\times10^5)\\[8pt]
   \max\!\left[\dfrac{0.0592\,Re^{*\,-0.2}}{T^*/T},\ 0.0256\left(\dfrac{R_s}{x_s}\right)^{0.2}\left(\dfrac{T^*}{T}\right)^{-0.65}\right] & (Re^*\ge5\times10^5)
   \end{cases}

後者の第 2 項は、粗さで決まる下限の局所値である（乱流域のみ）。:doc:`barrowman` の下限 :math:`0.032\,(R_s/L)^{0.2}`\ （全長 :math:`L` の平均値）は、
:math:`c_f\propto x_s^{-0.2}` の分布を 0 から :math:`L` まで平均すると :math:`1/(1-0.2)=1.25` 倍になるので、局所値 :math:`0.0256\,(R_s/x_s)^{0.2}` の平均と一致する。

摩擦による軸力係数は、パネルの面積 :math:`A_i` に、機軸方向の接線成分 :math:`\sqrt{1-n_x^2}` を掛けて

.. math::

   C_{f}^{\mathrm{panel}}=\frac{1}{S}\sum_i c_{f,i}\,A_i\sqrt{1-n_{x,i}^2}

と足す。尾部フェアリングは除く。サンプル機体の濡れ面積は、胴体が 0.4405 m\ :math:`^2`\ （抽出した胴体の 0.4408 m\ :math:`^2` と 0.07 % 以内）、フィンを含めて 0.5598 m\ :math:`^2` である。

確認済み
--------

実装が式どおりに動くことは、自動テストと :ref:`sec-vaero-panel` で確かめている。ここでは結論だけを挙げる。

* 解析解のある基本形状（球、長球、円すい、くさび）で、表面圧力や衝撃波角などが解析解と一致する。
* 胴体のみの法線力は細長体理論に近く、フィンの揚力は矩形翼の理論値の近くにある。
* メッシュを細かくしても法線力はほとんど変わらず、後流を十分長くしても変わらない。
* 亜音速から超音速までの係数に、遷音速の補間をまたいだ跳びがない。
* 部品積み上げ法との差は、量ごとに :ref:`sec-vaero-compare-panel` に示している。これは手法どうしの相互比較であり、実測との比較ではない。

注意
----

* 扱う流れは非粘性・渦なしで、実測（風洞試験や飛翔試験）との比較はまだない。
* 遷音速は補間であり、遷音速の物理は計算していない。
* 低い超音速の小迎角では、局所傾斜法の法線力が大きく出る（サンプルの :math:`M=1.2` で、迎角 2〜4° の :math:`C_N` が Euler の CFD より 22〜23 % 大きい。:ref:`sec-lim-panel`）。:math:`C_{N\alpha}` のピークはここにある。
* 尾部フェアリングは、後端の剥離領域を固体で置き換えた近似である。
* ロール項は部品積み上げ法のモデルの値で、パネルの解からは求めていない。カントはメッシュに反映されない。
* 超音速の翼端やフィンどうしの干渉の扱いは粗い。フィンと胴体の干渉は胴体の吹き上げだけで、フィンから胴体への持ち上げと、ノーズの後ろの円筒部の揚力（細長体の傾きでは 0）を含まない。底面・フィン縁・追加抗力は経験式である。
* 仮定と制約の全体は :ref:`sec-lim-panel` に、手法の使い分けは :ref:`sec-aero-select` にある。
