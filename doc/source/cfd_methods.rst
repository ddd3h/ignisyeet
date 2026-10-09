.. _page-theory-cfd-coefficients:

CFD の係数表
============

CFD の解は、Mach 数、迎角、横滑り角の点ごとの力とモーメントである。飛翔計算が読むのは、これを規則的な格子に並べた係数表なので、点から表を作る段階にも、結果を左右する取り決めがある。
迎角の正負をどう扱うか、CFD が与えない項を何で補うか、解いた範囲の外をどうつなぐかである。

この章は、まず CFD の値とモデルの値の区分を示し、迎角とケース行列の取り方、SU2 の係数から機体軸の係数への変換、軸対称表と一般表の作り方、解いた範囲の外、ロール減衰微係数の順に述べる。
最後に流れ場の図で係数の由来を確かめ、適用範囲と精度をまとめる。解く条件の選び方は :ref:`page-theory-cfd-choose`、表の定義は :ref:`page-theory-aero-tables` と :ref:`page-theory-aero-coefficients` にある。

.. _sec-cfd-supplied:

CFD の値とモデルの値
--------------------

係数表の各量が、CFD の解から来るか、部品積み上げ法（またはパネル法）のモデルから来るかを :numref:`tbl-cfd-supplied` と :numref:`fig-cfd-supplied` に示す。
モデルで補った量の精度は、部品積み上げ法の精度である。

.. _fig-cfd-supplied:

.. figure:: /_generated/tikz/cfd_supplied.*
   :width: 90%

   CFD の解から得る量とモデルで補う量（上）と、迎角の軸上の区分（下）。見るところは、解いた最大の迎角 :math:`\alpha_e` の先でモデルの値へつなぐことと、モデルで補う量の精度は部品積み上げ法の精度になることである。

.. _tbl-cfd-supplied:

.. list-table:: 係数表の量と出どころ
   :header-rows: 1
   :widths: 24 38 38

   * - 量
     - Euler
     - RANS
   * - 法線力 :math:`C_N`、圧力中心 :math:`x_{cp}`
     - CFD
     - CFD
   * - 軸力の圧力抗力（造波）
     - CFD
     - CFD
   * - 摩擦抗力
     - モデル
     - CFD（粗さの増分はモデル）
   * - 底面抗力
     - モデル（フェアリングの面は力に入れない）
     - ``rans_base`` で、モデル（既定）か CFD
   * - フィン縁、ボートテールの抗力
     - ``panel_mesh`` はモデル。``stl`` と ``step`` は縁の面が CFD の壁面にあるので CFD
     - ``panel_mesh`` は設定の縁の増分をモデルで足す。``stl`` は使えない。``step`` は CFD
   * - 横力 :math:`C_Y`、ヨーモーメント :math:`C_n`
     - 横滑りを解けば CFD
     - 横滑りを解けば CFD
   * - ロール減衰 :math:`C_{lp}`
     - ``roll_damping`` なら CFD、なければモデル
     - モデル
   * - ピッチ・ヨー減衰 :math:`C_{mq}`、:math:`C_{nr}`
     - 傾きと重心は CFD、ばらつきはモデル
     - 同左
   * - ロールのカント強制 :math:`C_{l\delta}`
     - モデル
     - モデル
   * - 解いた迎角と横滑り角の外側
     - モデル
     - モデル
   * - ロールとピッチ・ヨーの交差する微係数
     - どの手法でもモデルにない
     - 同左

迎角とケース行列
----------------

.. _sec-cfd-alpha-mode:

迎角の扱い
~~~~~~~~~~

``alpha_mode`` は、迎角の正負と :math:`\alpha=0` での非対称性の扱いを決める。
非構造のメッシュで解いた法線力 :math:`f(\alpha)` は、真の奇関数 :math:`f_{odd}(\alpha)` に、正負の迎角で同じ値だけ現れる数値的なずれ :math:`c_0` が足されたものになりうる（:math:`f(\pm\alpha)=\pm f_{odd}+c_0`）。
:numref:`fig-alpha-mode` は、三つの値がこのずれ :math:`c_0` をどう扱うかを示す。

.. _fig-alpha-mode:

.. figure:: /_generated/tikz/alpha_mode_schematic.*
   :width: 100%

   ``alpha_mode`` の三つの値。青の破線は :math:`f(\alpha)=f_{odd}+c_0`、橙は係数として使う量である。
   見るところは、``mirror`` は正負の両方を解いて :math:`c_0` を打ち消し、``offset`` は :math:`\alpha=0` の解で引き、``single`` は何もしないことである。

``"mirror"``\ （既定）
   各正の迎角を負の迎角でも解いて :math:`c_0` を打ち消す（:math:`z` 対称メッシュのときは鏡像なので解かない）。負の迎角のケースが収束することが前提である。
``"offset"``
   設定した迎角と :math:`\alpha=0` だけを解き、:math:`\alpha=0` の法線力とモーメントを引く。ずれが全迎角で同じ大きさであることが前提である。
``"single"``
   設定した迎角だけを解き、結果をそのまま使う。形状とメッシュが対称で、:math:`\alpha=0` で法線力が 0 になることが前提である。

形状が鏡映対称でない（ランチラグなど）とき、または片側の迎角だけ解きたいときは、既定の ``"mirror"`` から変える。
``"mirror"`` では、各正の迎角 :math:`\alpha` について、:math:`+\alpha` と :math:`-\alpha` の結果から

.. math::
   :label: eq-cfd-mirror

   C_N=\frac{C_N(+\alpha)-C_N(-\alpha)}{2},\qquad
   m=\frac{m(+\alpha)-m(-\alpha)}{2},\qquad
   C_A=\frac{C_A(+\alpha)+C_A(-\alpha)}{2}

を使う。:math:`m=C_N x_{cp}` はノーズ周りのモーメント（長さの次元）である。:math:`c_0` は :eq:`eq-cfd-mirror` の差で打ち消される。
片方の迎角の解が得られなかったときは、もう一方に :math:`\alpha=0` の値をずれの推定値として使う。
:math:`\alpha=0` の解の軸力 :math:`C_A` だけを使い、その法線力とモーメント（真値は 0）は、計算で現れた非対称性として報告に記録される。
``"offset"`` では、:math:`\alpha=0` の値を :math:`C_N^{(0)},m^{(0)}` として

.. math::
   :label: eq-cfd-offset

   C_N=C_N(\alpha)-C_N^{(0)},\qquad m=m(\alpha)-m^{(0)}

とする。``z_mirror_mesh`` が有効なとき、:math:`-\alpha` の解は :math:`+\alpha` の解の鏡像なので、``"mirror"`` の組合せは :math:`+\alpha` の結果に一致する。
:math:`\alpha=0` のずれは 0 になり、負の迎角の計算は不要になる。

一般表では、``"mirror"`` と ``"offset"`` のどちらでも、:math:`\alpha=\beta=0` の横力、法線力、ピッチとヨーのモーメント（:math:`C_Y,C_Z,C_m,C_n`）を全点から引く（:ref:`sec-cfd-general`）。
ロールモーメント :math:`C_l` は、鏡映対称な形状でだけ引く。鏡映対称でない形状では、カントしたフィンのロールの強制力なので残す。
鏡映対称でない形状では、その細部が :math:`\alpha=\beta=0` で生む小さな力も一緒に引かれる。``"single"`` では、形状の対称性で 0 にならない成分がそのまま残る。

.. _sec-cfd-choose-matrix:

ケース行列
~~~~~~~~~~

``machs``、``alphas_deg``、``betas_deg`` の組合せがケースの行列である。既定は、Mach 数 0.3、0.6、0.8、0.95、1.1、1.3、1.6、2.0、2.5、3.0 の 10 点と、迎角 0、2、4、8、12、16° の 6 点で、横滑り角は 0 だけである。
迎角は 0 を含む必要があり、横滑り角は 0 から始める。

* **Mach 数**\ ：飛翔が通る Mach 数の範囲を覆う。解いた最小と最大の Mach 数の外では、係数表は外挿せず端の値で一定になり、警告が出る。遷音速（:math:`M\approx0.8`\ 〜\ 1.2）は係数の変化が大きいので、この帯に点を置く。
* **迎角**\ ：飛翔で到達する迎角の上限まで解く。解いた最大の迎角 :math:`\alpha_e` の先は、その 2 倍（上限 90°）まで部品積み上げ法の値へなめらかにつなぐ（:eq:`eq-cfd-beyond`）。
* **ケース数**\ ：:math:`n_\alpha` 個の正の迎角と :math:`n_\beta` 個の正の横滑り角を指定したとき、1 つの Mach 数の ``"none"`` のケース数は :math:`(2n_\alpha+1)(2n_\beta+1)` である。対称性を使えば、解かない符号が減る（:ref:`sec-cfd-stl-symmetry`）。``roll_damping`` は Mach 数ごとに 2 ケースを足す。

ケースの並び順は、Mach 数が小さい順、各 Mach 数の中で横滑り角、その中で迎角である。
付属の ``examples/sample_cfd.toml`` は 12 ケース、``examples/sample_rans.toml`` は :math:`M=0.3,0.8,1.2,2` と :math:`\alpha=0,\pm4,\pm8^\circ` の 20 ケースである。

ロール減衰と横滑りは、必要なときだけ解く。

* ``roll_damping = true`` にすると、各 Mach 数で :math:`\alpha=\beta=0`、:math:`\hat p=\pm` ``roll_rate_hat`` の 2 ケースを、機体が回る流れとして全領域で解く（:ref:`sec-cfd-sideslip`）。Euler だけで使える。2 ケースの差からロール減衰微係数 :math:`C_{lp}` を求め、係数表に入れる（:ref:`sec-cfd-roll-post`）。差を取るので、フィンのカントや非対称による回転なしのずれは打ち消される。指定しないときの :math:`C_{lp}` は部品積み上げ法の値である。
* ``betas_deg`` に 0 以外を入れると横力 :math:`C_Y` とヨーモーメント :math:`C_n` が CFD から得られるが、係数表が一般表になり、ケースが増える。

.. _sec-cfd-table-kind:

係数表の種類
~~~~~~~~~~~~

CFD が作る係数表には、軸対称表と一般表（6 成分、:ref:`sec-aero-coeff6`）の二種類がある。次のどれかのとき、一般表になる。

* ``surface = "stl"`` か ``"step"`` である。
* ``shape_symmetry`` が ``"pitch_plane"`` か ``"none"`` と明示されている。
* ``betas_deg`` に 0 以外がある。

それ以外は軸対称表で、ロール減衰だけを解いた場合も軸対称表である。
一般表は、:math:`C_X,C_Y,C_Z,C_l,C_m,C_n` を (M, α, β) の関数として持つ 6 成分の準定常空力の表である（:ref:`sec-aero-table`）。
迎角は :math:`-180^\circ` から :math:`180^\circ`、横滑り角は :math:`-90^\circ` から :math:`90^\circ` の格子で、解いた範囲の外はモデルの値で埋める（:ref:`sec-cfd-region-outside`）。

解から係数へ
------------

.. _sec-cfd-post:

SU2 の係数から機体軸の係数へ
~~~~~~~~~~~~~~~~~~~~~~~~~~~~

機体はメッシュに固定されているので、SU2 の力の係数 :math:`C_{Fx},C_{Fz}` はそのまま機体軸の係数である。

.. math::
   :label: eq-cfd-body

   C_N=C_{Fz},\qquad C_A^{\mathrm{CFD}}=C_{Fx},\qquad m=C_N\,x_{cp}=-C_{My}\,d

SU2 はモーメントを :math:`\bm{M}=\bm{r}\times\bm{F}` で計算するので、その :math:`y` 成分は :math:`zF_x-xF_z` である。
ノーズ先端から後方 :math:`x_{cp}` に働く正の法線力は負の :math:`C_{My}` を与える。圧力中心は

.. math::
   :label: eq-cfd-xcp

   x_{cp}=\frac{m}{C_N}=-\frac{C_{My}\,d}{C_N}

である。:math:`|C_N|` が小さいときは定義しない。:math:`C_L,C_D` しか得られないときは、自由流の方向で回転して :math:`C_{Fx}=C_D\cos\alpha-C_L\sin\alpha`、:math:`C_{Fz}=C_D\sin\alpha+C_L\cos\alpha` とする。

.. _sec-cfd-body6:

6 成分の係数
~~~~~~~~~~~~

横滑りやロールを解くときは、SU2 の 6 成分の力とモーメントの係数 :math:`C_{Fx},C_{Fy},C_{Fz},C_{Mx},C_{My},C_{Mz}` を使う。
CFD の座標は機体座標を :math:`y` まわりに 180 度回したもの（:math:`\mathrm{diag}(-1,1,-1)`）で、モーメントは軸性ベクトルなので、機体軸の係数（:eq:`eq-coeff-6`）は

.. math::
   :label: eq-cfd-body6

   C_X=-C_{Fx},\quad C_Y=C_{Fy},\quad C_Z=-C_{Fz},\qquad
   C_l=-C_{Mx},\quad C_m=C_{My},\quad C_n=-C_{Mz}

となる。力は :math:`qS`、モーメントはノーズ先端まわりで :math:`qSd` で無次元化している。
:eq:`eq-cfd-body` との関係は、:math:`C_Z=-C_N`、:math:`C_m=x_{cp}C_Z/d` である（:eq:`eq-coeff-6-axisym`）。
半領域の解では、:math:`y=0` の対称面が半分の機体の圧力を受けるので、:math:`C_Y,C_l,C_n` は対称面による見かけの値である。機体は :math:`y=0` について対称なので、半領域のケースではこれらを 0 にする。
鏡映の像は、:math:`z\to-z` で :math:`(\alpha,\beta)\to(-\alpha,\beta)` となり :math:`C_Z,C_l,C_m` の符号が変わる。
:math:`y\to-y` では :math:`(\alpha,\beta)\to(\alpha,-\beta)` となり :math:`C_Y,C_l,C_n` の符号が変わる。

軸力
~~~~

軸力 :math:`C_A` は、CFD が供給しない項を部品積み上げ法で補う。
:math:`\alpha`・Mach 数ごとの CFD の軸力 :math:`C_A^{\mathrm{CFD}}` に、:eq:`eq-cd0` の成分のうち CFD が与えない項を、迎角の補正 :math:`f(\alpha)`\ （:eq:`eq-ca-alpha`）をかけて足す。

* **Euler**\ ：CFD は圧力による抗力（造波抗力）だけを与える。底面は力の積分から外してあるので、

  .. math::
     :label: eq-cfd-ca

     C_A=\max\Bigl(C_A^{\mathrm{CFD}}+f(\alpha)\bigl(C_{D,f}+C_{D,\mathrm{base}}+C_{D,\mathrm{fin}}+C_{D,\mathrm{bt}}+C_{D,\mathrm{extra}}\bigr),\,0\Bigr)

  である。足すのは摩擦、底面、フィン縁（丸い前縁と四角い後縁）、ボートテール、追加抗力で、ノーズの圧力抗力 :math:`C_{D,\mathrm{nose}}` は **含めない**\ （CFD が与えるため）。
  ``surface = "stl"`` と ``"step"`` では、フィンの前縁と後縁の面が CFD の壁面にあり、その圧力抗力は :math:`C_A^{\mathrm{CFD}}` にすでに入っているので、フィン縁の項 :math:`C_{D,\mathrm{fin}}` を足さない（足すと二重に数える）。
  ``panel_mesh`` の形状は、抽出した厚みのフィンを作り直すので、この項を足す。
  ``"stl"`` と ``"step"`` で、フィンの後縁が底面にそろっている（根元と翼端の後縁がどちらも底面の平面 :math:`x=L` から :math:`10^{-4}L` 以内にある）と、後縁の面は ``base`` の境界に入って力に積分されない。その底面抗力を次の項で足す。

  .. math::
     :label: eq-cfd-te-flush

     C_{D,\mathrm{TE,flush}}=\frac{C_{D,\mathrm{base}}(M)\,n\,b\,t}{S_{\mathrm{ref}}}

  ここで :math:`C_{D,\mathrm{base}}(M)` は部品積み上げ法の底面抗力係数、:math:`n` はフィンの枚数、:math:`b` はスパン、:math:`t` は抽出した平均のフィンの厚さである。
  この項は :eq:`eq-cfd-ca` の和に入れ、``fin_te_square = false`` のとき、後縁が底面から内側にあるとき、``panel_mesh`` のときは 0 である。
  平均の厚さを使うので、後縁が鈍いくさび形のとき、実際の底面抗力より小さい。燃焼中の値では、底面の項に、ノズル出口の面積を除いた底面積を使う。

* **RANS**\ ：壁面の軸力 :math:`C_{Fx}(\mathrm{wall})` は、圧力と摩擦、フィンの縁とボートテールの力を含む。底面の項は ``rans_base`` で選び、

  .. math::
     :label: eq-cfd-ca-rans

     C_A=\max\Bigl(C_{Fx}(\mathrm{wall})+C_{A,\mathrm{base}}+f(\alpha)\bigl(C_{D,\mathrm{extra}}+\Delta C_{D,\mathrm{rough}}+\Delta C_{D,\mathrm{fin}}\bigr),\,0\Bigr),\qquad
     C_{A,\mathrm{base}}=\begin{cases}f(\alpha)\,C_{D,\mathrm{base}} & (\text{rans\_base = model})\\ C_{Fx}(\mathrm{base}) & (\text{rans\_base = cfd})\end{cases}

  とする。``"model"`` は部品積み上げ法の底面抗力（燃焼中はノズル出口の面積を除いた底面積）、``"cfd"`` は RANS が平らな底面に積分した力である。
  ``"model"`` で後縁がそろっていれば、:eq:`eq-cfd-te-flush` を :math:`f(\alpha)` をかけて足す。``"cfd"`` では足さない（後縁の面は ``base`` の力に入っている）。
  ``"cfd"`` の燃焼中の値は、ノズル出口の面積にかかる底面圧力の寄与を引く。
  RANS の壁は滑らかなので、設定の粗さの効果を、部品積み上げ法の粗い壁と滑らかな壁の摩擦の差として足す。:math:`C_{D,f}` は粗さ :math:`k_s` の摩擦（:eq:`eq-cd0`）、:math:`c_f` は摩擦係数の式で、同じ Reynolds 数での比を使い、

  .. math::
     :label: eq-cfd-rans-rough

     \Delta C_{D,\mathrm{rough}}=C_{D,f}\Bigl(1-\frac{c_f(Re,M,0)}{c_f(Re,M,k_s/L)}\Bigr)

  とする（粗さ 0 なら 0）。
  作り直した形状（``panel_mesh``）では、フィンの前縁と後縁を弦長の割合 :math:`e=0.02` で鈍くしてあり（:ref:`sec-cfd-rebuild`）、その縁の面の厚さは最大の厚さの :math:`4e(1-e)=7.8` % である。
  設定の縁（``fin_le``、``fin_te``）の抗力は縁の厚さに比例するとして、部品積み上げ法のフィン縁の項 :math:`C_{D,\mathrm{fin}}` から、鈍くした縁がすでに壁面の力に持つ分を引いて

  .. math::
     :label: eq-cfd-rans-fin

     \Delta C_{D,\mathrm{fin}}=C_{D,\mathrm{fin}}\bigl(1-4e(1-e)\bigr)

  を足す。``surface = "step"`` では縁の面が CFD の壁面にあるので :math:`\Delta C_{D,\mathrm{fin}}=0` である。
  どちらの増分もモデルによる補正で、CFD の解ではない。部品積み上げ法の摩擦とボートテールの項は足さない。

亜音速の Euler では、CFD の軸力の理論値は 0（d'Alembert のパラドックス）である。実際の値は小さな負の数（サンプル機体の迎角 0〜8° で :math:`-0.01` から :math:`-0.05`）になり、そのまま合計に入る。

ピッチ・ヨー減衰
~~~~~~~~~~~~~~~~

定常な CFD はピッチ減衰を与えない。そこで :ref:`sec-damping` の和 :math:`S_0,S_1,S_2`\ （:ref:`page-theory-barrowman` の :eq:`eq-body-sums` と同じ定義）は、
部品積み上げ法（パネル法があればその値）の **ばらつき** を残したまま、全体の傾きと重心を CFD の値に取り替える。
モデルの和を :math:`S_k^{m}` として、そのまわりのばらつき（分散）

.. math::
   :label: eq-cfd-var

   \sigma^2=\frac{S_2^m}{S_0^m}-\left(\frac{S_1^m}{S_0^m}\right)^2

を保ち、CFD の :math:`C_{N\alpha}` と小迎角の圧力中心 :math:`x_{cp0}` から

.. math::
   :label: eq-cfd-damp

   S_0=C_{N\alpha},\qquad S_1=C_{N\alpha}\,x_{cp0},\qquad S_2=C_{N\alpha}\bigl(x_{cp0}^2+\sigma^2\bigr)

とする。これらの和から :math:`C_{mq}`、:math:`C_{nr}` が決まる関係は :ref:`sec-aero-damping` にある。CFD が直接与えるのは傾きと重心位置だけである。

係数表の作成
------------

.. _sec-cfd-table-build:

軸対称表
~~~~~~~~

解いた点は不規則な格子（解いた Mach 数と迎角）なので、それを係数表の規則的な格子に埋める。

1. 各 Mach 数の列で、:math:`C_N` とノーズ周りのモーメント :math:`m` は迎角について奇関数とし、原点 :math:`(0,0)` を通る。
   :math:`C_N,m` と CFD の軸力を、迎角について単調な 3 次補間（PCHIP :cite:`fritsch`）する。
   解いた最大の迎角 :math:`\alpha_e` より大きい表の行は、部品積み上げ法へつなぐ（下記）。端の傾きは延長しない。
2. :math:`C_{N\alpha}` と :math:`x_{cp}(\alpha\to0)` は、最小の二つの正の迎角を通る奇関数の 3 次式 :math:`c_1\alpha+c_3\alpha^3` から求める（当てはめた傾き :math:`c_1` が最小の迎角での割線の傾きから 5 割を超えてずれるときは、割線の傾きを使う）。
   :math:`x_{cp0}` は、モーメント :math:`m` に同じ当てはめをした傾きを :math:`C_{N\alpha}` で割った値である。
   手順 1 の :math:`C_N` と :math:`m` の補間は、原点での傾きをこの当てはめの傾きにして始める。
   こうすると、表の :math:`C_{N\alpha}`、迎角 0 の圧力中心 :math:`x_{cp0}`、減衰の和（:eq:`eq-cfd-damp`）と、表の :math:`C_N` と :math:`m` の行が、一つの曲線から求めたものになる。
3. Mach 数の方向にも、各量を係数表の Mach 数の格子に PCHIP で補間する。解いた Mach 数の範囲の外には外挿せず、最小または最大の Mach 数の値で一定とする。
   迎角 0 の圧力中心は、モーメントの傾き :math:`C_{N\alpha}x_{cp0}` と :math:`C_{N\alpha}` をそれぞれ補間した比とする（:math:`|C_{N\alpha}|\le10^{-9}` では :math:`x_{cp0}` を補間する）。
   0 でない迎角の行が、補間したモーメントを補間した :math:`C_N` で割った圧力中心を持つ（:eq:`eq-xcp-interp`）のと同じ扱いで、解いた Mach 数の間でも、迎角 0 の圧力中心と減衰の和が小さい迎角の行と食い違わない。
   途中の Mach 数の列が失敗していれば、両隣の列で補間して埋める。最小または最大の Mach 数の列の失敗は補えず、エラーになる。
4. 軸力に上の項を足し、ピッチ減衰の和を取り替える。``roll_damping`` が真のときは、:math:`C_{lp}` も CFD の値（:ref:`sec-cfd-roll-post`）にする。

解いた迎角より大きい範囲には CFD のデータがない。端の傾きを延長した直線は、最後の区間で圧力中心が動く向きに圧力中心を際限なく動かし、大きな迎角で重心より前に出すことがある。
そうなると大きな迎角に見かけの安定な釣り合い（トリム）ができ、頂点のあとの弾道落下が、遅く遠くまで流される落下になる。
そこで、Mach 数の列ごとに、:math:`\alpha_e` での CFD の値（:math:`C_{N,e}`、:math:`x_{cp,e}=m_e/C_{N,e}`、:math:`C_{A,e}`）から部品積み上げ法の値（添字 B）へ移す。
部品積み上げ法のフィンの法線力は大迎角で有界で（:ref:`sec-fin-large-alpha`）、フィンの圧力中心は平面形の図心へ動く。

.. math::
   :label: eq-cfd-beyond

   C_N(\alpha) &= C_{N,B}(\alpha)\,\bigl(1+(r-1)\,w(\alpha)\bigr),\qquad r=\frac{C_{N,e}}{C_{N,B}(\alpha_e)}\\
   x_{cp}(\alpha) &= x_{cp,B}(\alpha)+\bigl(x_{cp,e}-x_{cp,B}(\alpha_e)\bigr)\,w(\alpha)\\
   C_A(\alpha) &= C_{A,e}\,\frac{f(\alpha)}{f(\alpha_e)}

ここで :math:`f` は部品積み上げ法の軸力の迎角の倍率（:eq:`eq-ca-alpha`）、:math:`w` は :math:`\alpha_e` で 1、:math:`\alpha_r=\min(2\alpha_e,\,90^\circ)` で 0 になる、両端の傾きが 0 の 3 次 Hermite 補間で、:math:`\alpha\ge\alpha_r` では 0 である。
:math:`r` が正でないとき（:math:`C_{N,e}` と :math:`C_{N,B}(\alpha_e)` の符号が違う）は、比の代わりに差を使い :math:`C_N=C_{N,B}(\alpha)+(C_{N,e}-C_{N,B}(\alpha_e))\,w(\alpha)` とする。
:math:`x_{cp}` は機体の内側（0 から全長）に制限する。
値は :math:`\alpha_e` で CFD の値に一致し、:math:`\alpha_r` から先は部品積み上げ法の法線力と圧力中心になる（軸力は CFD の水準に部品積み上げ法の形を掛けたもので、:math:`90^\circ` で 0）。
このつなぎは ``extrapolation`` の設定によらない。``extrapolation`` は表に記録され、表の格子の外の引き出しだけに効く（:ref:`sec-aero-interp`）。
``examples/sample_cfd.toml``\ （解く迎角 0、4、8°、``alpha_max_deg = 90``）では、8° から 16° の間でつなぎ、16° から 90° は部品積み上げ法の値になる。

.. _sec-cfd-general:

一般表
~~~~~~

一般表（6 成分、:ref:`sec-aero-coeff6`）になる条件は :ref:`sec-cfd-table-kind` にある。
格子は、迎角が :math:`-180^\circ` から :math:`180^\circ`、横滑り角が :math:`-90^\circ` から :math:`90^\circ` で、どちらも非一様である。:math:`\pm30^\circ` までは 2 度刻み、その外側は 10 度刻み（迎角 61 点、横滑り角 43 点）とする。
Mach 数の刻みは ``general_mach_step``\ （既定 0.1、0 なら ``[aero]`` の ``mach_step``）で、範囲は ``[aero]`` の ``mach_min`` から ``mach_max`` である。
1 つの Mach 数あたり :math:`61\times43` 点で、Mach 数の刻みを粗くするとファイルが小さくなる（ファイルの形式は :ref:`sec-file-general`）。

Mach 数ごとの作り方は次のとおりである。

1. **対称性による補完**\ 。解いた点に、形状の鏡映（:ref:`sec-cfd-body6`）の像を加える。``alpha_mode = "mirror"`` では、同じ :math:`(\alpha,\beta)` に重なるすべての像を平均し、数値的な非対称を消す。
   そうでなければ、解いた点が像より優先される。``"mirror"`` と ``"offset"`` では、:math:`(0,0)` の点の :math:`C_Y,C_Z,C_m,C_n` を、先に全点から引く。
   :math:`C_l` は鏡映対称な形状でだけ引き、鏡映対称でない形状ではカントしたフィンのロールの強制力として残す。
   像のない成分（鏡映対称でない形状のすべて、``"pitch_plane"`` の :math:`C_Z,C_m`）は、この引き算で取り除かれる（:ref:`sec-cfd-val-asym-cause`）。
   そのあとで、形状の対称性から厳密に 0 になる成分を 0 にする。:math:`y=0` について鏡映対称な形状では、:math:`\beta=0` の点の :math:`C_Y,C_l,C_n` を 0 にし、:math:`z=0` についても対称な形状（``"axisymmetric"``）では、さらに :math:`\alpha=0` の点の :math:`C_Z,C_l,C_m` を 0 にする。
   ``"none"`` では解の値をそのまま使う。
2. **CFD の補間**\ 。解いた範囲 :math:`R=[\alpha_{\min},\alpha_{\max}]\times[\beta_{\min},\beta_{\max}]` の中を、各横滑り角で迎角について PCHIP :cite:`fritsch` し、その値を横滑り角について PCHIP する（変数分離）。
3. **モデルで埋める**\ 。:math:`R` の外側は、部品積み上げ法の軸対称表を 6 成分に展開した値 :math:`T`\ （:eq:`eq-coeff-6-axisym`、全迎角 90 度まで）の変化を、:math:`R` の境界の CFD の値に合わせて足す。
   :math:`R` の内側に最も近い点を :math:`(\alpha_c,\beta_c)` として、

   .. math::
      :label: eq-cfd-fill

      C(\alpha,\beta)=F(\alpha_c,\beta_c)+T(\alpha,\beta)-T(\alpha_c,\beta_c)

   とする。境界をまたいで連続である。:math:`R` の内側では :math:`C=F` である。
4. **軸力**\ 。CFD の :math:`C_A^{\mathrm{CFD}}=-F_{C_X}` に、:eq:`eq-cfd-ca` と同じ項を足す。迎角の補正は :math:`(\alpha_c,\beta_c)` の全迎角 :math:`\cos\alpha_t=\cos\alpha\cos\beta` から :math:`\alpha_t'=\min(\alpha_t,\pi-\alpha_t)` で決める。燃焼中と燃焼後の :math:`C_X` が得られる。
5. **Mach 数の方向**\ 。格子の点ごとに、解いた Mach 数の値を PCHIP で補間する。解いた範囲の外は外挿せず、端の値で一定とする（``extrapolation`` にかかわらない）。最小または最大の Mach 数のケースが全部失敗していれば、補えないのでエラーにする。

ピッチとヨーの減衰の和は、零角度の傾きから別々に取り替える。
ピッチは :math:`\beta=0` で、ヨーは :math:`\alpha=0` で、奇関数の 3 次式 :math:`c_1\theta+c_3\theta^3` を最小の二つの正の角度に当てはめて傾きを求め、

.. math::
   :label: eq-cfd-slopes

   \begin{aligned}
   C_{N\alpha}^{q}&=-\frac{\partial C_Z}{\partial\alpha},&\quad
   x_{cp0}^{q}&=d\,\frac{\partial C_m/\partial\alpha}{\partial C_Z/\partial\alpha},\\
   C_{N\alpha}^{r}&=-\frac{\partial C_Y}{\partial\beta},&\quad
   x_{cp0}^{r}&=-d\,\frac{\partial C_n/\partial\beta}{\partial C_Y/\partial\beta}
   \end{aligned}

とする。:eq:`eq-cfd-damp` をピッチ（:math:`S^q`）とヨー（:math:`S^r`）にそれぞれ適用する。
横滑りを解かないときは、ヨーの和をピッチの和に等しくする。ピッチとヨーを別々に求めるので、機体が非対称なときは異なる値になる。

ロールのカント強制 :math:`C_{l\delta}` は、部品積み上げ法の値を使う。
一般表は、表の :math:`C_l` に含まれるカント :math:`\delta_{\mathrm{ref}}` を記録する（:eq:`eq-cl-total`）。
STL モードでは、形状（STL）にカントが含まれているので、:math:`\delta_{\mathrm{ref}}` は設定したフィンのカントである。
カントの影響は :math:`C_l` にすでに入っており、:math:`\delta=\delta_{\mathrm{ref}}` の飛翔では強制項を足さない（二重に数えない）。
``panel_mesh`` は形状にカントがないので、:math:`\delta_{\mathrm{ref}}=0` である。

.. _sec-cfd-region-outside:

CFD の領域の外
~~~~~~~~~~~~~~

一般表の値のうち、解いた迎角と横滑り角の範囲の外側は、CFD の結果ではなくモデルの値である。
解いた範囲 :math:`R` の内側は CFD の補間値で、外側は :eq:`eq-cfd-fill` により境界をまたいで連続につないだモデルの値である。
展開は軸対称な形状のものなので、非対称な形状の :math:`R` の外側では当てにならない。
どこまでが CFD の値かは、実行の報告に出る解いた迎角と横滑り角の最小・最大で確かめる。係数表のファイルには領域の情報は付かない。

Mach 数の方向は異なり、解いた Mach 数の範囲の外は、``extrapolation`` にかかわらず、端の値で一定とする。
表の Mach 数の格子が解いた範囲より広いと、一定になる Mach 数が報告され、警告が出る。
軸対称表では、解いた最大の迎角より大きい行を部品積み上げ法へのつなぎ（:eq:`eq-cfd-beyond`）で埋める。
CFD が与えない項とピッチ・ヨー減衰の和は、部品積み上げ法（またはパネル法）のモデルから補う（:ref:`sec-cfd-post`）。

.. _sec-cfd-roll-post:

ロール減衰微係数
~~~~~~~~~~~~~~~~

回転座標系の 2 ケース（:ref:`sec-cfd-sideslip`）から、Mach 数ごとに

.. math::
   :label: eq-cfd-clp

   C_{lp}=\frac{C_l(+\hat p)-C_l(-\hat p)}{2\,\hat p}

を求める（:math:`C_l=-C_{Mx}`）。差を取るので、カントや非対称による零回転のずれは消える。
次の場合、その Mach 数の値は使わず、部品積み上げ法の :math:`C_{lp}` に替えて、理由を警告として報告する。

* 2 ケースのどちらかが失敗したか、値が有限でない。
* フィンがあるのに :math:`C_{lp}` が正である（ロール減衰は負でなければならない）。
* フィンがあるのに :math:`|C_{lp}|<10^{-6}` である。

表の値は、部品積み上げ法の :math:`C_{lp}(M)` に、使える Mach 数での CFD / 部品積み上げ法の比を PCHIP で補間して（範囲外は端の値で一定）かけた値である（部品積み上げ法の値が 0 に近いときは、CFD の値そのものを補間する）。
使える Mach 数がなければ部品積み上げ法の値になる。報告には、すべての Mach 数が CFD か、解かない（またはすべて不採用）か、混在かが示される。

.. _sec-cfd-flow:

流れ場を見る
------------

係数表の値は、流れ場の積分である。流れ場を見ると、法線力、圧力中心、軸力が壁面のどこから生まれるかがわかる。
サンプル機体の :math:`\alpha=4^\circ`、Euler、``wall_size`` 2 mm（フィンの面 1 mm、142 万セル）、一様流からの開始で解いたときの図を示す。データは ``doc/data/cfd/fields/`` に保存してある。

:numref:`fig-cfd-mach-field` は対称面 :math:`y=0` での Mach 数の分布である。
:math:`M_\infty=0.8` では、ノーズ先端によどみ点（青）があり、ノーズの肩で流れが加速する（橙）。
:math:`M_\infty=2.0` では、ノーズ先端から斜めに広がる青い帯がノーズの衝撃波で、迎角があるので風上側（:math:`z<0`）のほうが強い。フィンの前縁からも衝撃波が出ている。

.. _fig-cfd-mach-field:

.. figure:: /_generated/plots/cfd_mach_field.*
   :width: 100%

   対称面 :math:`y=0` での Mach 数の分布（:math:`\alpha=4^\circ`、上は :math:`M_\infty=0.8`、下は :math:`M_\infty=2.0`）。
   流れは左から右へ向かう。色の中心は一様流の Mach 数で、青が一様流より遅い領域、橙が速い領域を表す。範囲を超える値は端の色で、灰色は機体の内部である。

:numref:`fig-cfd-cp-surface` は壁面の圧力係数 :math:`C_p` の分布で、半モデルの解を :math:`y=0` 面で鏡映して全機を描いた。機体の下側（風上側）から見ている。
ノーズでは風上側の :math:`C_p` が風下側より高く、この差が法線力を生む。
先端から 0.02 から 0.25 m の範囲で、風上側（:math:`z<0`）と風下側（:math:`z>0`）の半分ごとに面積で重み付けした平均は、:math:`M_\infty=0.8` で風上側 :math:`-0.015`、風下側 :math:`-0.069`、:math:`M_\infty=2.0` で風上側 0.104、風下側 0.045 である。
:math:`M_\infty=0.8` では、尾部フェアリングの付け根（底面の縁）で流れが曲がって加速し、環状の負圧ができる（最小で :math:`C_p\approx-0.9`）。

.. _fig-cfd-cp-surface:

.. figure:: /_generated/plots/cfd_cp_surface.*
   :width: 100%

   壁面の圧力係数（:math:`\alpha=4^\circ`、上は :math:`M_\infty=0.8`、下は :math:`M_\infty=2.0`）。左はノーズ、右はフィンと尾部フェアリング。ノーズの風上側と風下側の色の差と、フィンの縁の分布を見る。色の範囲は図ごとに異なる。

これらの図は、係数表の値が壁面の圧力分布を積分した結果であることを示す。法線力は風上側と風下側の圧力差、軸力の圧力抗力はノーズとフィンの前縁の圧力、底面の負圧は Euler では底面抗力のモデルで補う部分に当たる。

適用範囲と精度
--------------

* **解く内容**\ ：定常解を (M, α, β) の点ごとに並べた準定常の表である。角速度に依存する非定常な空力を、すべて表現するものではない。ピッチ・ヨー減衰は傾きと重心だけが CFD で、ばらつきはモデルである。ロールとピッチ・ヨーの交差する微係数は扱わない。
* **Euler**\ ：粘性による摩擦とはく離を含まない。平らな底面は解が定まらないので、``panel_mesh`` では尾部フェアリング、``stl`` と ``step`` では ``base_treatment`` で扱う。底面、摩擦、フィン縁の抗力は経験式による。亜音速の軸力の理論値は 0 で、実際の値は小さな負の数になる。
* **RANS**\ ：``panel_mesh``\ （フィンの配置が :math:`y=0` について対称なときだけ）と ``step`` で使える。SA は一方程式の渦粘性モデルで、付着した流れの摩擦を対象に検証した。底面の後流の圧力、はく離の大きい流れは確かめていない。
* **一般表と領域の外**\ ：解いた迎角と横滑り角の外側はモデルの値で、非対称な形状ではあてにならない。軸対称表は、解いた最大の迎角の 2 倍（上限 90°）から先が部品積み上げ法の値になる。Mach 数は外挿せず、端の値で一定とする。
* **形状**\ ：``panel_mesh`` は回転体と台形フィンに限られる。``stl`` と ``step`` は、フィンの縁の面を ``fin_size`` で解像した場合に限り、縁の圧力抗力が軸力に入る。フィンの後縁が鈍いくさび形のとき、そろった後縁の底面抗力は過小になる。
* **メッシュ**\ ：メッシュ収束は完全でなく、フィンの面を 1 mm より細かくした計算はない。STL のメッシュは作成のたびに少し変わる。
* **ロール減衰**\ ：Euler の解で、フィンの摩擦を含まない。符号とほぼ 0 の検査に通らない Mach 数は部品積み上げ法の値になる。

実装上の制約の一覧は :ref:`sec-lim-cfd` にある。

.. _sec-cfd-trust:

どこまで信頼してよいか
~~~~~~~~~~~~~~~~~~~~~~

CFD が与える値は、法線力 :math:`C_N`、圧力中心 :math:`x_{cp}`、軸力のうち圧力による抗力（RANS では摩擦も）、横滑りを解けば横力 :math:`C_Y` とヨーモーメント :math:`C_n`、``roll_damping`` ならロール減衰 :math:`C_{lp}` である。
それ以外はモデルで補う（:numref:`tbl-cfd-supplied`）。検証と実測との比較を、スコープを区別して示す。

* **検証**\ （解析解、相関式、収束との比較）：
  円すいの表面圧力は、メッシュを細かくすると Taylor–Maccoll の解 :cite:`taylormaccoll` に近づく（:ref:`sec-vaero-cfd`）。
  表の :math:`C_{N\alpha}` は法線力の補間曲線の原点での傾きと 0.1 % 以内で一致する。
  :math:`z` 対称メッシュと ``alpha_mode = "mirror"`` で :math:`\alpha=0` の見かけの非対称性が消える（:ref:`sec-cfd-val-asym`）。ロール減衰と横滑りの符号と無次元化、6 成分への写像は、軸対称表を展開した値と一致する（:ref:`sec-cfd-val-roll`）。
  メッシュ収束は、フィンの面を 1 mm にしたまま ``wall_size`` を変えると :math:`C_N` が単調に増える（:ref:`sec-vaero-cfd-mesh`）。
  RANS は、平板の摩擦が NASA TMR の SA の解と 0.8 % 以内（:math:`M=0.2`）、:math:`M=2` の van Driest II と 1.2 % 以内（最も細かい格子）で一致し、円すいとフィンなし胴体の摩擦の軸力も平板の見積もりと 2.5〜7.4 % で一致する。
  サンプル機体の RANS の :math:`C_N`\ 、:math:`C_A`\ 、:math:`x_{cp}`\ （:math:`M=0.8,2`、:math:`\alpha=4^\circ`）は、メッシュに対して単調に収束する（GCI は :math:`C_N` で 0.5 % 以内、:math:`C_A` で 1.9 % 以内）。
* **相互比較**\ （手法どうし）：
  亜音速の :math:`C_{N\alpha}` は、CFD とパネル法の差が 2.5 %（:math:`M=0.8`）から 9 %（:math:`M=0.3`）で、部品積み上げ法より CFD に近い（:ref:`sec-vaero-compare`）。
  STL をそのまま解く計算は、作り直した形状による計算と相互比較した（:ref:`sec-cfd-val-stl`）。
  RANS は Euler の CFD と :math:`C_N` で 3.6〜5.1 %、:math:`x_{cp}` で 4 mm 以内で一致する（形状の違いを含む。:ref:`sec-cfd-rans-compare`）。
* **妥当性確認**\ （実測との比較）：
  Basic Finner の超音速（Mach 1.08–4.30）の射場試験との比較だけで、Euler の :math:`C_{N\alpha}` は実測より 3〜11 % 小さく、:math:`x_{cp}` は 0.2 口径以内で合う（:ref:`page-vv-validation-basic-finner`）。

RANS の検証の項目と結果は :numref:`tbl-rans-gates`\ （:ref:`sec-cfd-rans`）にあり、次の条件は確かめていない、または基準を満たさない。

* 亜音速の :math:`\alpha=0` の非対称：:math:`M=0.8` で :math:`C_N(0)` が 0.011〜0.034 残る（底面の後ろの剥離した流れの非対称な解）。``alpha_mode = "mirror"`` が :math:`\pm\alpha` の組合せで取り除く。
* 低い亜音速：:math:`M=0.3` の :math:`\alpha=0,-4^\circ,-8^\circ` は残差が周期的に振動し、収束しない。サンプル機体の係数表（20 ケース）で収束と判定されたのは 15 ケース（75 %）で、基準の 90 % に届かない。
* 底面の流れ：底面の圧力（``rans_base = "cfd"``）は確かめていない。
* CAD のモデル：``surface = "step"`` の RANS はメッシュの検査までを確かめ、SU2 の解は確かめていない。
* 壁関数：``yplus`` 30〜150 は検証していない。

検証の全体は :ref:`page-vv-cfd-verification` と :ref:`page-vv-rans-verification`、手法どうしの相互比較は :ref:`page-vv-cross-comparison`、計算時間とメモリの測定は :ref:`page-vv-performance` にある。

関連する設定
------------

迎角と Mach 数の格子は ``machs``、``alphas_deg``、``betas_deg``、``alpha_mode``、``alpha_max_deg``、ロール減衰は ``roll_damping`` で決める。
底面の力を CFD から取るかは ``rans_base``、一般表の Mach 数の刻みは ``general_mach_step`` で決める。
``extrapolation`` は表の格子の外の引き出しだけに効き、CFD の領域の外のつなぎには効かない。
解く条件の設定は :ref:`page-theory-cfd-choose` にある。各キーの型、既定値、値の範囲は :ref:`page-reference-config-cfd` にある。
