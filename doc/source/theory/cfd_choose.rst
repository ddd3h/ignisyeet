.. _page-theory-cfd-choose:

CFD の選び方
==============

本章は、``aero.method = "cfd"`` で係数表を作るときに設定ファイルで選ぶ項目（流れのモデル、壁面の形状、対称性、迎角の扱い、メッシュの大きさ、資源）について、何を選ぶとどうなるかと、選ぶ根拠を示す。
CFD は、機体のまわりの空間を格子で覆い、圧縮性流れの方程式を数値的に解く。
部品積み上げ法（:ref:`page-theory-barrowman`）とパネル法（:ref:`page-theory-panel`）が流れ場を解かずに係数を求めるのに対し、衝撃波、遷音速の流れ、フィンと胴体の干渉を方程式から直接得られる。
その代わり、1 ケースが数分から数十分、行列全体で数十分から数時間かかり、メモリも使う。
手法そのものの選び方は :ref:`page-theory-aero-choose` にある。方程式は :ref:`page-theory-cfd-equations`、結果から係数表を作る式は :ref:`page-theory-cfd-coefficients` に述べる。
ツールの用意は :ref:`page-howto-cfd-setup`、実行の手順は :ref:`page-howto-cfd-table`、CAD のモデルを使う手順は :ref:`page-howto-cad-step` にある。

この章で決めること
--------------------

.. list-table::
   :header-rows: 1
   :widths: 20 38 42

   * - 決めること
     - 設定キー
     - 本章の節
   * - 流れのモデル
     - :ref:`aero.cfd.model <cfg-aero-cfd-model>`\ （``"euler"``、``"rans"``）
     - :ref:`sec-cfd-choose-model`、:ref:`sec-cfd-rans-use`
   * - 壁面の形状
     - :ref:`aero.cfd.surface <cfg-aero-cfd-surface>`\ （``"panel_mesh"``、``"stl"``、``"step"``）、:ref:`step_file <cfg-aero-cfd-step_file>`、:ref:`step_unit <cfg-aero-cfd-step_unit>`、:ref:`stl_remesh <cfg-aero-cfd-stl_remesh>`
     - :ref:`sec-cfd-surface`
   * - 底面の扱い
     - :ref:`base_treatment <cfg-aero-cfd-base_treatment>`、:ref:`tail_fairing <cfg-aero-cfd-tail_fairing>`、:ref:`rans_base <cfg-aero-cfd-rans_base>`
     - :ref:`sec-cfd-stl-base`、:ref:`sec-cfd-rans-use`
   * - 対称性と計算領域
     - :ref:`shape_symmetry <cfg-aero-cfd-shape_symmetry>`、:ref:`symmetry <cfg-aero-cfd-symmetry>`、:ref:`z_mirror_mesh <cfg-aero-cfd-z_mirror_mesh>`、:ref:`farfield <cfg-aero-cfd-farfield>`
     - :ref:`sec-cfd-stl-symmetry`、:ref:`sec-cfd-domain`
   * - 迎角と横滑りの扱い
     - :ref:`alpha_mode <cfg-aero-cfd-alpha_mode>`、:ref:`machs <cfg-aero-cfd-machs>`、:ref:`alphas_deg <cfg-aero-cfd-alphas_deg>`、:ref:`betas_deg <cfg-aero-cfd-betas_deg>`、:ref:`roll_damping <cfg-aero-cfd-roll_damping>`
     - :ref:`sec-cfd-alpha-mode`、:ref:`sec-cfd-choose-matrix`
   * - メッシュの大きさ
     - :ref:`wall_size <cfg-aero-cfd-wall_size>`、:ref:`fin_size <cfg-aero-cfd-fin_size>`、:ref:`yplus <cfg-aero-cfd-yplus>`、:ref:`max_geometry_error <cfg-aero-cfd-max_geometry_error>`
     - :ref:`sec-cfd-mesh-size`、:ref:`sec-cfd-prism`
   * - 反復と収束
     - :ref:`iterations <cfg-aero-cfd-iterations>`、:ref:`min_iterations <cfg-aero-cfd-min_iterations>`、:ref:`stable_window <cfg-aero-cfd-stable_window>`、:ref:`convergence <cfg-aero-cfd-convergence>`、:ref:`cfl <cfg-aero-cfd-cfl>`、:ref:`scheme <cfg-aero-cfd-scheme>`、:ref:`multigrid <cfg-aero-cfd-multigrid>`
     - :ref:`sec-cfd-int-convergence`
   * - 資源
     - :ref:`ranks_per_case <cfg-aero-cfd-ranks_per_case>`、:ref:`parallel_cases <cfg-aero-cfd-parallel_cases>`、:ref:`threads と memory_gb <cfg-resources-memory_gb>`
     - :ref:`sec-cfd-resources`、:ref:`sec-cfd-parallel`

選び方
--------

流れのモデルと壁面の形状は、ほかの選択の前提になる。次の二つの流れ図と表で先に決める。
それ以外の項目は、既定のままで検証した範囲に収まるように値を決めてあるので、既定から変えるのは、表 :numref:`tbl-choose-cfd-other` の理由があるときに限る。

.. _sec-cfd-choose-model:

Euler か RANS か
~~~~~~~~~~~~~~~~

:numref:`fig-choose-cfd-model` は、``model`` を選ぶ流れである。
最初の問いが決め手で、軸力の摩擦の寄与を、部品積み上げ法の式でなく解として得たいときだけ RANS を考える。
Euler は粘性を含まず、壁面の力は圧力だけで決まる。摩擦抗力、底面抗力、フィン縁の抗力は CFD からは得られず、部品積み上げ法の項で補う（:ref:`sec-cfd-post`）。
RANS は乱流を Spalart–Allmaras モデルで表し、壁面の摩擦も解に入る。

.. _fig-choose-cfd-model:

.. figure:: /_generated/tikz/cfd_model_choice.*
   :width: 90%

   ``model`` の選び方。左の列は問いで、上から順に答える。右の箱は、その問いで Euler に戻る場合の対処である（橙は Euler に戻る場合、緑は Euler を選ぶ場合）。
   見るところは、RANS に進むには、壁面、フィンの配置、メモリ、Mach 数と迎角の範囲の四つの条件がすべて合う必要があることである。
   一つでも外れる場合は Euler を使う。

.. _tbl-choose-cfd-model:

.. list-table:: Euler と RANS の比較（サンプル機体。数値の条件は各行に示す）
   :header-rows: 1
   :widths: 18 41 41

   * - 項目
     - Euler（既定）
     - RANS
   * - 表すもの
     - 非粘性の圧縮性流れ（:eq:`eq-cfd-euler`）。衝撃波、遷音速の流れ、フィンと胴体の干渉を含む
     - Euler に粘性と乱流（Spalart–Allmaras、:eq:`eq-cfd-sa`）を加えたもの。付着した流れの摩擦と、底面の後流を含む
   * - 軸力のうち CFD が与える項
     - 圧力による抗力（造波抗力）。摩擦、底面、フィン縁、ボートテールは部品積み上げ法で補う
     - 壁面の圧力と摩擦。底面は ``rans_base`` で、部品積み上げ法（既定）か CFD かを選ぶ。粗さとフィンの縁の効果は部品積み上げ法の増分として足す（:eq:`eq-cfd-ca-rans`）
   * - 使える壁面
     - ``panel_mesh``、``stl``、``step``
     - ``panel_mesh``\ （フィンの配置が :math:`y=0` について対称なときだけ）と ``step``
   * - 検証した範囲
     - 円すいの表面圧力（Taylor–Maccoll の解、:math:`M=2`）、サンプル機体の :math:`M=0.8` と 2、:math:`\alpha=4^\circ` のメッシュ収束、手法どうしの相互比較。実測との妥当性確認は Basic Finner の超音速だけ（:ref:`sec-cfd-trust`）
     - 平板の摩擦（NASA TMR の SA の解と 1 % 以内、:math:`M=2` の van Driest II と 1.5 % 以内）、サンプル機体の :math:`M=0.8` と 2、:math:`\alpha=\pm4^\circ` の :math:`C_N`\ 、:math:`C_A`\ 、:math:`x_{cp}`\ （メッシュ収束と Euler との相互比較）。項目ごとの結果は :numref:`tbl-rans-gates`
   * - 確かめていない、または基準を満たさない条件
     - 平らな底面（``base_treatment = "flat"``）は収束しないことがある。亜音速の軸力は小さな負の値になる（:math:`-0.01`\ 〜\ :math:`-0.05`、迎角 0〜8°）
     - :math:`M=0.3` の :math:`\alpha=0,-4^\circ,-8^\circ` は収束しない。:math:`M=0.8` の :math:`\alpha=0` は底面の後流の非対称が :math:`C_N` に 0.01〜0.03 残る。底面の圧力、CAD のモデルの解、:math:`M=0.3,1.2` と :math:`\alpha=8^\circ` のメッシュ収束は未確認
   * - 1 ケースのメモリ
     - 約 40 MB × ランク数 + 3.3 kB × セル数（:eq:`eq-cfd-mem`）。既定のメッシュ（約 140 万セル、4 ランク）で約 4.8 GB の見積もり
     - 約 40 MB × ランク数 + 12 kB × 節点数（:eq:`eq-cfd-mem-rans`）。``wall_size`` 5 mm（73.5 万節点）で約 9 GB、3 mm で約 22 GB の見積もり
   * - 1 ケースの時間
     - 4 ランクで約 6〜15 分（既定のメッシュ。:ref:`sec-cfd-resources`）
     - 5 mm で約 15〜60 分（4 ランク。:ref:`sec-cfd-resources`）
   * - 選ぶとき
     - 係数表を短時間で作りたいとき。摩擦と底面は部品積み上げ法の項で足りるとき。STL をそのまま解くとき
     - 摩擦抗力を CFD の解で得たいとき。検証した範囲（:math:`M=0.8`\ 〜\ 2、:math:`|\alpha|\le4^\circ` 付近）が飛翔の主な範囲のとき
   * - 避けるとき
     - 摩擦抗力の大きさが結果を左右するのに、部品積み上げ法の摩擦の式を信頼できないとき
     - 低い亜音速が主な範囲のとき（検証しておらず、:math:`M=0.3` は収束しないケースがある）。メモリが足りないとき。結果を使う前に確かめる時間がないとき

測定値の出典は :ref:`sec-vaero-timing`\ （時間とメモリ）と :ref:`sec-cfd-rans`\ （RANS の検証）である。
Euler と RANS の相互比較（:math:`C_N` が 3.6〜5.1 %、:math:`x_{cp}` が 4 mm 以内）は :ref:`sec-cfd-rans-compare` にある。

壁面の形状（panel_mesh、stl、step）の選び方
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

:numref:`fig-choose-cfd-surface` は、``surface`` を選ぶ流れである。
入力が STL だけなら ``panel_mesh`` か ``stl``、CAD のモデルがあれば ``step`` も選べる。

.. _fig-choose-cfd-surface:

.. figure:: /_generated/tikz/cfd_surface_choice.*
   :width: 90%

   ``surface`` の選び方。上から順に答える。見るところは、レールボタンなどの付属物が結果に効くかどうかと、CAD のモデルがあるかどうかの二つが分岐を決めることである。
   破線は、``stl`` と ``step`` を Euler で解くときに ``base_treatment`` を必ず指定する条件を示す。

.. _tbl-choose-cfd-surface:

.. list-table:: 壁面の形状の比較（サンプル機体、Euler。セル数は ``doc/data/cfd/fin_resolution.csv``）
   :header-rows: 1
   :widths: 18 27 27 28

   * - 項目
     - ``panel_mesh``\ （既定）
     - ``stl``
     - ``step``
   * - 壁面が表すもの
     - 抽出した半径分布とフィンの寸法から作り直した回転体と台形フィン
     - 入力の STL を修復し、重なった殻を一つの立体にして再メッシュしたもの
     - CAD のモデルの立体。曲面は厳密な曲面のままメッシュにする
   * - 入らない形状
     - レールボタン、ポッド、非対称な突起
     - なし（STL にあるものはすべて入る）
     - なし（CAD にあるものはすべて入る）
   * - 入力との同一性
     - 同一でない（抽出した寸法から作り直す）
     - 同一でない（再メッシュとフェアリング）。ずれを忠実度の報告で確かめる
     - CAD の曲面そのもの。``rocket.stl`` との位置合わせで同じ機体と確かめる
   * - 使える流れのモデル
     - Euler、RANS
     - Euler のみ
     - Euler、RANS
   * - 係数表の種類
     - 軸対称表（横滑りやカントを解くときは一般表）
     - 一般表
     - 一般表
   * - 対称性の使い方
     - フィンの配置から決める。対称なら 1/4 領域
     - 形状の検査に通った平面だけを使う
     - 形状の検査に通った平面だけを使う
   * - 1 ケースのセル数
     - ``wall_size`` 6 mm、フィンの面 1 mm の半モデルで 81 万
     - 5 mm、フィンの面 1 mm の全領域で 182 万
     - 6 mm、フィンの面 1 mm の全領域で 162 万
   * - 選ぶとき
     - 機体が回転体と台形フィンで表せ、付属物の影響が小さいとき。最も軽い
     - 付属物を含めて解きたいが CAD がないとき
     - 付属物を含めて解きたく、CAD のモデルがあるとき。RANS も使いたいとき
   * - 避けるとき
     - 付属物の抗力や横力が係数に効くとき
     - RANS が要るとき。忠実度の報告で壁面が入力から大きくずれたとき
     - CAD のモデルが ``rocket.stl`` と別の版、単位、向きのとき（位置合わせで止まる）

その他の選択
~~~~~~~~~~~~

次の項目は、既定のままで検証した範囲に収まる。値を変える理由と、変えたときの影響を示す。

.. _tbl-choose-cfd-other:

.. list-table:: 既定から変える理由
   :header-rows: 1
   :widths: 20 30 50

   * - 項目
     - 変えるとき
     - 影響
   * - ``alpha_mode``
     - 形状が鏡映対称でない（ランチラグなど）とき、または片側の迎角だけ解きたいとき
     - 既定の ``"mirror"`` は :math:`\pm\alpha` から奇関数部分を取り出す。``"offset"`` は :math:`\alpha=0` のずれを引く。``"single"`` は解をそのまま使う（:ref:`sec-cfd-alpha-mode`）
   * - ``shape_symmetry``、``symmetry``
     - 全領域で解いて対称性の仮定を外したいとき
     - ``"none"`` は全領域でケース数とセル数が増える。``"auto"`` は検査に通った平面だけを使う（:ref:`sec-cfd-stl-symmetry`）
   * - ``machs``、``alphas_deg``
     - 飛翔の Mach 数と迎角の範囲に合わせる
     - ケース数が行列の大きさで決まる（:ref:`sec-cfd-choose-matrix`）
   * - ``betas_deg``、``roll_damping``
     - 横滑りやロール減衰を CFD で得たいとき
     - 係数表が一般表になり（横滑り）、ケース数が増える（:ref:`sec-cfd-table-kind`）
   * - ``wall_size``、``fin_size``
     - 時間とメモリに余裕があり精度を上げたいとき、または資源が足りないとき
     - セル数、時間、:math:`C_N` が変わる（:ref:`sec-cfd-mesh-size`）
   * - ``yplus``
     - RANS で壁関数を使いたいとき
     - 5 以下は壁面まで解像する。30〜150 は壁関数で、検証していない。5 より大きく 30 より小さい値と 150 より大きい値はエラーである（:ref:`sec-cfd-prism`）
   * - ``farfield``
     - 通常は変えない
     - 10〜40 倍の全長で :math:`C_N` は 0.25 % 以内で一致する（:ref:`sec-cfd-domain`）
   * - ``iterations``、``min_iterations``、``stable_window``、``convergence``
     - ケースが ``unconverged`` や失敗になるとき
     - 反復の上限、早期終了の最小反復数と窓、残差の目標が変わる。既定は測定で決めてある（:ref:`sec-cfd-int-convergence`）
   * - ``multigrid``
     - 変える理由はない
     - サンプルのメッシュでは粗い格子が作られず、設定しても結果も時間も変わらない
   * - ``ranks_per_case``、``parallel_cases``
     - 使える CPU とメモリに合わせる
     - 1 ケースの速さと同時に解けるケース数が変わる（:ref:`sec-cfd-parallel`）

モデル
--------

.. _sec-cfd-surface:

壁面の形状
~~~~~~~~~~~~

壁面は、CFD が解く機体の表面である。:numref:`fig-cfd-surface-modes` のように、抽出した寸法から立体を作り直す ``panel_mesh``、入力の STL を修復して使う ``stl``、CAD のモデルの立体をそのまま使う ``step`` がある。
``panel_mesh``\ （``"cad"`` も同じ意味）と ``stl`` の壁面は入力の STL と同一ではない。``step`` の壁面は CAD の曲面そのもので、Euler で ``base_treatment = "fairing"`` としたときだけ底面の後ろが変わる。
飛翔計算の形状（基準面積、全長、フィンの寸法、部品積み上げ法の項）は、どの壁面でも ``rocket.stl`` から抽出する。

.. _fig-cfd-surface-modes:

.. figure:: /_generated/tikz/cfd_surface_modes.*
   :width: 100%

   壁面の三つのモード。(a) ``panel_mesh`` は抽出した半径分布とフィンの寸法から立体を作り直すので、入力にあるレールボタンなどは含まれない。
   (b) ``stl`` は入力の STL を修復して一つの立体にし、壁面を再メッシュする。(c) ``step`` は CAD の曲面をそのままメッシュにする。
   見るところは、``panel_mesh`` と ``stl`` の壁面が入力と同一でなく、``stl`` ではずれを忠実度の報告で確かめることである。

.. _sec-cfd-rebuild:

panel_mesh の形状
~~~~~~~~~~~~~~~~~

``surface = "panel_mesh"`` の入力は、:ref:`page-theory-geometry` で抽出した胴体の半径分布 :math:`r(x)` とフィンの寸法である。
これらから、回転体の胴体と、両凸断面の台形フィン（根元翼弦、先端翼弦、後退、スパン、厚み）を一つの立体として作り直す。
曲面が正確な曲線として残るので、メッシュは曲率に応じて切り直される。
形状は回転体と台形フィンに限られ、レールボタン、ポッド、非対称な突起は入らない。それらを含めて解くときは ``stl`` か ``step`` を使う。
フィンは、既定ではロールが 0 度で :math:`\pm y` と :math:`\pm z` の 4 方向に付く。``fin_roll_deg`` でフィンの組を機軸のまわりに回転できる（この設定は ``panel_mesh`` だけで使い、45 度ではフィンが :math:`y=0` 面に載らない）。

RANS では、フィンの前縁と後縁を少し鈍くする。両凸断面の厚さの分布 :math:`4\xi(1-\xi)`\ （:math:`\xi` は翼弦に沿った無次元の位置）のうち :math:`\xi\in[e,\,1-e]`、:math:`e=0.02` の部分を翼弦の全長に引き伸ばし、縁を短い直線で閉じる。
縁の厚さは最大の厚さの :math:`4e(1-e)=7.8` % になる。鋭い縁ではプリズム層が折れ重なるためで、Euler の形状は変えない。
鈍くした縁が壁面の力に持つ分は、軸力で補正する（:eq:`eq-cfd-rans-fin`）。

.. _sec-cfd-stl:

STL を直接解く
~~~~~~~~~~~~~~

``surface = "stl"`` は、STL をそのまま CFD の壁面にするモードである。
STL は、サンプル機体のようにフィンが胴体に埋め込まれた別の閉じた立体として表されていることがあり、そのままでは体積メッシュにできない。
そこで、STL の殻を修復して一つの立体にし、その表面を再メッシュして壁面とする。
抽出した寸法には依存しないので、``fin_roll_deg`` は使わず、STL に含まれるフィンの向き（ロール）もそのまま保つ。
修復は点の位置を動かさず、頂点の溶接、縮退した三角形の除去、向きの統一だけを行う。

次の入力はエラーになる。水密でない殻、体積のない殻、そして殻の和が互いに離れた複数の立体になる場合（フィンが胴体に触れていないなどの入力の誤り）である。

壁面の再メッシュは ``stl_remesh`` で選ぶ。``"classify"``\ （既定）は、曲面を作って ``wall_size`` の等方的な三角形に作り直す（フィンの面は ``fin_size``）。
``"none"`` は和の三角形をそのまま使うが、サンプルの STL ではフィンの面が少数の大きな三角形になってフィンを解像できず、法線力が 3 割ほど低くなる。

**CFD の壁面は入力と同一ではない。** 殻の和、壁面の再メッシュ、Euler の底面のフェアリングで、壁面は入力の STL から変わる。
ずれの大きさは ``wall_size`` の程度に収まるのが普通だが、保証ではない。結果を使う前に、形状の忠実度の報告（:ref:`sec-cfd-fidelity`）で確かめる。

.. _sec-cfd-step-internals:

CAD のモデルを解く
~~~~~~~~~~~~~~~~~~

``surface = "step"`` は、``step_file`` に書いた CAD のモデル（STEP、IGES、BREP）の立体を、そのまま壁面にする。
ランチラグ、レールボタン、カメラのポッド、ノズルのように STL の抽出では扱えない細部も、CAD にあれば壁面に入る。
曲面は三角形に近似せず、厳密な曲面のままメッシュにするので、``stl`` のような再メッシュによるずれはない。
書き出しの条件（単位、一つの閉じた立体、座標系）と報告の読み方、エラーへの対処は :ref:`page-howto-cad-step` にある。

この壁面が成り立つ前提は次の三つである。

* CAD のモデルは、``rocket.stl`` と同じ機体である。両者を同じ座標系で照合し、合わなければメッシュを作る前にエラーにする（下の表）。
* 飛翔計算の寸法と部品積み上げ法の項（底面抗力、摩擦抗力、Euler で補う項）は ``rocket.stl`` の形状から求める。CAD の細部の圧力抗力は CFD の軸力に入るので、``rocket.extra_cd`` で二重に数えない。
* Euler の摩擦抗力は STL の形状から求めるので、細部の摩擦抗力は入らない。

.. list-table:: 同じ機体とみなす条件（:math:`L` は STL から抽出した全長、:math:`h_w` は ``wall_size``）
   :header-rows: 1
   :widths: 24 76

   * - 条件
     - 内容
   * - 包含箱
     - STL の包含箱が CAD のモデルの包含箱に :math:`0.01L` の余裕で収まる。CAD のモデルの包含箱は、どの軸でも STL の 1.25 倍を超えない
   * - ノーズ先端
     - :math:`x` の最小値の差が :math:`0.01L` 以下である
   * - 面の距離
     - STL の面上の点が、すべて CAD のモデルの面から :math:`\max(2h_w,\,0.01L)` 以内にある。CAD の立体の内側に :math:`\max(h_w,\,0.002L)` より深く埋まった点（胴体に差し込んだ箱で表したフィンなど）は除き、除いた点は STL の点の 25 % 以下である
   * - CAD にだけある部分
     - STL の面から :math:`\max(h_w,\,0.002L)` より離れた部分は、CAD の細部として許し、つながった領域の数と面積を報告する

サンプル機体（:math:`L=1.5` m）を ``wall_size`` 2 mm で照合すると、許容は 15 mm、細部とみなす距離は 3 mm である。
許容は形の大きな食い違い（単位、向き、別の版のモデル）を見つけるためのもので、:math:`\max(2h_w,\,0.01L)` より小さい形の違い（フィンの厚さの違いなど）は通る。
不一致の理由は、両方の包含箱の最も長い辺の比から、単位、向きか位置、別の版のモデルのどれかに推定してエラーに添える。

.. _sec-cfd-stl-base:

底面の扱い
~~~~~~~~~~

Euler の底面は、平らなままでは解が定まらないので、``base_treatment`` と ``tail_fairing`` で扱いを決める。
STL と CAD のモデル（``surface = "step"``）を Euler で解くときは、``base_treatment`` を **必ず** 指定する（指定がなければエラーになる）。

.. _sec-cfd-fairing:

尾部フェアリング
~~~~~~~~~~~~~~~~

非粘性の流れでは、平らな底面の後ろの流れは問題として成り立たない。後流が非対称な定常解を持ち、解の対称性が保てないためである。
そこで Euler では、底面の後ろに円すい形の尾部フェアリングを置く。長さは ``tail_fairing``\ （既定 6、単位は底面半径）で、底面半径の 10 % で切って平らな面で閉じる。
フェアリングの面は ``base`` の境界で、力の積分から外す。したがって底面の抗力は CFD からは得られず、部品積み上げ法の底面抗力の項で補う（:ref:`sec-cfd-post`）。
RANS では後流が解の一部なので、底面は平らなままにする。

``"fairing"``
   底面の後ろに尾部フェアリングを加える。**入力の形状を変更する** ので、警告を出す。
   ``surface = "step"`` では、最も大きい平らな底面の後ろに円すいを CAD の立体と融合する。**CAD の形状を変更する** ので、同じく警告を出す。
   フィンの後縁が底面と同一平面にそろっているとき（Basic Finner）は、フィンの後縁も底面の一部になって力の積分から外れる。その底面抗力は、抽出した平均のフィンの厚さを使って部品積み上げ法の底面抗力で補う（式は :eq:`eq-cfd-te-flush`）。
   平均の厚さを使うので、後縁が鈍いくさび形のときは抗力を過小に見積もる。

``"flat"``
   形状を変えず、平らな底面を残す。ただし、この Euler の解は収束しないことがある。サンプルの STL では、残差が :math:`10^{-2.5}` 付近で止まり、力の係数 :math:`C_{Fz}` が反復 400 回付近で符号を変えて振動した。
   警告を出し、収束しないケースは失敗として扱う。

``panel_mesh`` では、Euler のとき ``tail_fairing`` の長さでフェアリングを置く（0 にすると平らな底面）。
RANS の ``panel_mesh`` と ``step`` は ``base_treatment`` を使わず、平らな底面のまま解く（指定しても無視する）。``surface = "stl"`` では RANS を使えない。

.. _sec-cfd-fidelity:

形状の忠実度の報告
~~~~~~~~~~~~~~~~~~

``stl`` では、CFD の壁面が入力の形からどれだけずれたかを報告する。再メッシュとフェアリングで形が変わるので、結果の信頼性の目安になる。
報告は、入力、殻の和、CFD の壁面の三つの面について、包含箱、三角形の数、表面積、（定義できるものは）体積を示す。
入力は殻が重なるので体積を定義しない。

距離の最大値のうち大きいほうが ``wall_size`` の半分を超えると警告を出し、``max_geometry_error``\ （既定は ``wall_size``）を超えるとエラーにする。
最大値の位置も記録されるので、ずれが意図した変更（フェアリングが底面とつながる位置など）か、再メッシュの失敗かを判断できる。
報告は ``ignisyeet cfd-check 設定ファイル --mesh`` が表示し、``cfd_report.json`` の ``fidelity`` にも入る。

サンプルの STL を ``wall_size`` 2 mm、``base_treatment = "fairing"`` で解いたときは、CFD の壁面は和から ``wall_size`` より小さい距離にある。
入力からの最大のずれはフェアリングが底面とつながる位置にあって、意図した変更である。この最大値は ``wall_size`` の半分を超えるので警告が出るが、99 パーセンタイルは ``wall_size`` よりはるかに小さい。
三つの面の表と距離の表は :ref:`sec-vaero-cfd-fidelity` に示す。

.. _sec-cfd-int-fidelity:

忠実度の距離の意味
~~~~~~~~~~~~~~~~~~

面どうしの距離は、サンプリングによる近似で測り、最大値、99 パーセンタイル、二乗平均平方根を示す。比較は次の二つである。

* **CFD の壁面と和**\ ：両方向の距離。メッシュが元の面からずれた量を表す。
* **CFD の壁面と入力**\ ：壁面から入力への片方向の距離。入力に胴体へ埋まったフィンの箱の面があるので逆向きは意味を持たず、フェアリングは入力にないので ``base`` は含めない。

99 パーセンタイルと最大値の差が大きいときは、ずれが局所的である。距離はサンプリングによる近似で、細い特徴の見落としがありうる。

.. _sec-cfd-stl-symmetry:

対称性と計算領域の種類
~~~~~~~~~~~~~~~~~~~~~~

機体が鏡映対称なら、解く領域を半分、または 1/4 にして、計算量を減らせる。計算領域には次の種類がある。

* **全領域**\ ：機体全体を囲む領域。横滑り角の負側やロール減衰のケースは、全領域でなければ解けない。
* **半領域**\ ：機体の片側（:math:`y\ge0`）だけを囲み、:math:`y=0` の平面に対称の境界を置く。セル数は全領域の約半分になる。
* **1/4 領域**\ ：:math:`y\ge0` かつ :math:`z\ge0` の領域。メッシュを作る途中の領域で、:math:`z=0`\ （と必要なら :math:`y=0`）で鏡映して半領域または全領域にしてから解く。

半領域を使うのは、``symmetry = true``\ （既定）で、かつ、形状が :math:`y=0` について対称で、``shape_symmetry`` に :math:`y=0` の鏡映が含まれ、``betas_deg = [0]`` で、ロール減衰のケースがないときだけである。
どれか一つでも崩れると全領域を解く。``symmetry = false`` は全領域を強制する。

``shape_symmetry`` は、鏡映像として得られるケースを解かずに省く対称性を指定する。

.. list-table::
   :header-rows: 1
   :widths: 26 74

   * - 値
     - 動作
   * - ``"auto"``\ （既定）
     - ``panel_mesh`` では、横滑りを解かないとき ``"axisymmetric"``\ 。横滑りを解くときは、フィンの配置から決める（両面について対称なら ``"axisymmetric"``、:math:`y=0` だけなら ``"pitch_plane"``、それ以外は ``"none"``）。
       ``stl`` と ``step`` では、検査に通った平面だけを使う（両方なら ``"axisymmetric"``、:math:`y=0` だけなら ``"pitch_plane"``、それ以外は ``"none"``）。:math:`z=0` だけが通ったときは、使える形状の対称性がないので ``"none"`` とする
   * - ``"axisymmetric"``
     - :math:`y=0` と :math:`z=0` の両方について鏡映対称（フィンが 4 枚で ``fin_roll_deg`` が 0 度または 45 度など）。:math:`\alpha\ge0`、:math:`\beta\ge0` だけを解く
   * - ``"pitch_plane"``
     - :math:`y=0` だけについて鏡映対称（フィンが 3 枚など）。:math:`\pm\alpha` と :math:`\beta\ge0` を解く
   * - ``"none"``
     - 対称性を使わない。検査にかかわらず全領域で、:math:`\pm\alpha` と :math:`\pm\beta` を解く

``stl`` と ``step`` で ``"axisymmetric"`` か ``"pitch_plane"`` を明示したとき、形状が検査に通らなければ **エラー** にして、最大の距離と理由を示す。
非対称な形状を黙って鏡映して、鏡映像の係数を誤ることはない。``panel_mesh`` では検査をせず、明示した値をそのまま使うので、フィンの配置と合う値を指定する。
選んだ対称性と理由は、実行のとき標準エラー出力に表示し、``cfd_report.json``\ （``symmetry_check``）に記録する。

.. _sec-cfd-stl-symmetry-check:

鏡映対称の判定
~~~~~~~~~~~~~~

``stl`` と ``step`` は、形状が鏡映対称かを、機軸を含む二つの平面 :math:`y=0`\ （ピッチ面）と :math:`z=0` について別々に検査する。
鏡映した表面が元の表面から、全長の :math:`10^{-6}` 倍（サンプルで 1.8 μm）以内にあれば、その平面について対称と判定する。
三角形の分割が鏡映の関係にないだけなら対称と判定する。メッシュ生成器が曲面や平面を鏡映の関係にない三角形に分割するのは普通のことで、サンプルの STL でも三角形の 99 % は鏡映のものと一致しない。
CAD のモデルは、曲面そのものを厳密に調べ、B-spline の曲面の精度（Basic Finner の STEP で約 :math:`6\times10^{-7}` m）より細かくは判定しない。それでもランチラグやレールボタンのずれ（mm の大きさ）よりはるかに小さいので、非対称な形状は対称と判定されない。
CAD のモデルに対称な面からずれたランチラグやレールボタンがあると、形状は鏡映対称でなく、全領域を解く。付属の ``sample_rocket.step`` は対称でなく、全領域になる。

平面ごとに、対称か否か、最大の距離、非対称のときの理由を ``cfd_report.json`` の ``symmetry_check`` に記録する。サンプルの STL での結果は :ref:`sec-vaero-cfd-symmetry` に示す。
サンプルの STL では、検査に通れば 1/4 領域のメッシュが全領域より大幅に速く作れ、セル数も約 4 分の 1 になる。
同じメッシュから作った半領域と鏡映した全領域の係数は近いが、別々に作った二つのメッシュの比較は、メッシュの違いが混ざるので意味を持たない。

.. _sec-cfd-domain:

計算領域と遠方境界
~~~~~~~~~~~~~~~~~~

遠方境界は、機体の中央を中心とし、半径 :math:`R_\infty=` ``farfield`` × 全長（既定 20）の球の一部、または一辺 :math:`2R_\infty` の箱である。
領域の形は壁面の形状と対称性で次のように決まる。

.. list-table:: 遠方境界の形
   :header-rows: 1
   :widths: 34 66

   * - 場合
     - 領域
   * - ``panel_mesh``、半領域
     - 半球（:math:`y\ge0`）。:math:`y=0` の平面に対称の境界を課す
   * - ``panel_mesh``、全領域、フィンの配置が :math:`y=0` について対称
     - 半球を :math:`y=0` で鏡映した全球
   * - ``panel_mesh``、全領域、フィンの配置が対称でない（3 枚フィンなど）
     - 箱
   * - ``stl``、\ ``step``\ （Euler）
     - 対称と確かめた平面で切った 1/4 または半分の球（箱）を鏡映して半領域または全領域にする。対称でなければ箱の全領域
   * - RANS、半領域
     - 半分の箱（:math:`y\ge0`）
   * - RANS、全領域
     - 半分の箱を :math:`y=0` で鏡映した箱。``step`` で形状が対称でないときは、閉じた壁面のまわりの箱

境界には、``wall``\ （機体表面。力を積分する面）、``base``\ （底面。Euler ではフェアリングの面で力を積分しない。RANS では平らな底面で、軸力に使うかは ``rans_base`` で決める）、``farfield``\ （遠方境界）、``symmetry``\ （:math:`y=0` の平面。半領域のときだけ）の名前が付く。
物理的な意味は :ref:`sec-cfd-bc` にある。

``z_mirror_mesh``\ （既定 ``true``）は、機体が :math:`z` 方向にも鏡映対称（:math:`z\to-z`）であるときに効く。
非構造のメッシュは対称にならず、半モデルのメッシュを一度に作ると :math:`z>0` と :math:`z<0` でセルの並びが異なって、:math:`\alpha=0` でも法線力が 0 にならない（数値は :ref:`sec-vaero-cfd-symmetry` に示す）。
そこで :math:`y\ge0` かつ :math:`z\ge0` の 1/4 領域だけをメッシュにし、:math:`z=0` で鏡映して半球にする（:numref:`fig-cfd-domain`）。
こうして得たメッシュは :math:`z` について **厳密に対称** で、:math:`-\alpha` の解は :math:`+\alpha` の解の鏡像になる（:ref:`sec-cfd-alpha-mode`）。
形状が :math:`z\to-z` について対称でないとき（``panel_mesh`` でフィンが 3 枚や 5 枚、ロールが 10 度など）は、自動的に無視される。
RANS は半領域から作るので、``z_mirror_mesh`` は効かず、``alpha_mode = "mirror"`` では負の迎角も解く。

.. _fig-cfd-domain:

.. figure:: /_generated/tikz/cfd_domain.*
   :width: 100%

   計算領域の概念図。(a) 半球の :math:`y=0` 面での断面。機体は拡大して描き、実際の遠方境界は機体よりはるかに大きい。
   上半分（:math:`z\ge0`）だけをメッシュにし、:math:`z=0` で鏡映して下半分を作る。
   (b) 機軸の方向から見た半球。:math:`y=0` が対称の平面で、フィンが :math:`+y` と :math:`\pm z` に付いている。
   見るところは、メッシュを作る領域が 1/4 だけで、残りは鏡映で得ることである。

遠方境界の半径を 10、20、40 倍の全長に変えると（:math:`M=0.8`、:math:`\alpha=4^\circ`、``wall_size`` 2 mm）、:math:`C_N` は 0.25 % 以内で一致し、:math:`x_{cp}` は 20 倍と 40 倍で約 0.2 % 変わる。
壁面のメッシュの大きさを保ったまま遠方境界を広げても、セル数はほとんど増えない（10 倍から 40 倍で 0.3 % 増）。したがって既定の 20 倍を変える理由は通常ない。

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
Mach 数の刻みは ``general_mach_step``\ （既定 0.1、0 なら ``[aero]`` の ``mach_step``）で、粗くするとファイルが小さくなる。作り方は :ref:`sec-cfd-general` にある。

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

.. list-table:: ``alpha_mode`` の値
   :header-rows: 1
   :widths: 14 26 30 30

   * - 値
     - 解くケース
     - 除かれる誤差
     - 前提
   * - ``"mirror"``\ （既定）
     - 各正の迎角を負の迎角でも解く（:math:`z` 対称メッシュのときは鏡像なので解かない）
     - :math:`\alpha=0` のずれ :math:`c_0`
     - 負の迎角のケースが収束すること
   * - ``"offset"``
     - 設定した迎角だけと :math:`\alpha=0`
     - :math:`\alpha=0` の法線力とモーメント
     - ずれが全迎角で同じ大きさであること
   * - ``"single"``
     - 設定した迎角だけ
     - なし。結果をそのまま使う
     - 形状とメッシュが対称で、:math:`\alpha=0` で法線力が 0 になること

``"mirror"`` では、各正の迎角 :math:`\alpha` について、:math:`+\alpha` と :math:`-\alpha` の結果から

.. math::
   :label: eq-cfd-mirror

   C_N=\frac{C_N(+\alpha)-C_N(-\alpha)}{2},\qquad
   m=\frac{m(+\alpha)-m(-\alpha)}{2},\qquad
   C_A=\frac{C_A(+\alpha)+C_A(-\alpha)}{2}

を使う。:math:`m=C_N x_{cp}` はノーズ周りのモーメント（長さの次元）である。:math:`c_0` は :eq:`eq-cfd-mirror` の差で打ち消される。
片方の迎角の解が得られなかったときは、もう一方に :math:`\alpha=0` の値をずれの推定値として使う。
:math:`\alpha=0` の解の軸力 :math:`C_A` だけを使い、その法線力とモーメント（真値は 0）は、計算で現れた非対称性として ``cfd_report.json`` の ``asymmetry`` に記録する。
``"offset"`` では、:math:`\alpha=0` の値を :math:`C_N^{(0)},m^{(0)}` として

.. math::
   :label: eq-cfd-offset

   C_N=C_N(\alpha)-C_N^{(0)},\qquad m=m(\alpha)-m^{(0)}

とする。``z_mirror_mesh`` が有効なとき、:math:`-\alpha` の解は :math:`+\alpha` の解の鏡像なので、``"mirror"`` の組合せは :math:`+\alpha` の結果に一致する。
:math:`\alpha=0` のずれは 0 になり、負の迎角の計算は不要になる。

一般表では、``"mirror"`` と ``"offset"`` のどちらでも、:math:`\alpha=\beta=0` の横力、法線力、ピッチとヨーのモーメント（:math:`C_Y,C_Z,C_m,C_n`）を全点から引く（:ref:`sec-cfd-general`）。
ロールモーメント :math:`C_l` は、鏡映対称な形状でだけ引く。鏡映対称でない形状では、カントしたフィンのロールの強制力なので残す。
鏡映対称でない形状（ランチラグなど）では、その細部が :math:`\alpha=\beta=0` で生む小さな力も一緒に引かれる。``"single"`` では、形状の対称性で 0 にならない成分がそのまま残る。

.. _sec-cfd-choose-matrix:

ケース行列の決め方
~~~~~~~~~~~~~~~~~~

``machs``、``alphas_deg``、``betas_deg`` の組合せがケースの行列である。既定は、Mach 数 0.3、0.6、0.8、0.95、1.1、1.3、1.6、2.0、2.5、3.0 の 10 点と、迎角 0、2、4、8、12、16° の 6 点で、横滑り角は 0 だけである。
迎角は 0 を含む必要があり、横滑り角は 0 から始める。

* **Mach 数**\ ：飛翔が通る Mach 数の範囲を覆う。解いた最小と最大の Mach 数の外では、係数表は外挿せず端の値で一定になり、警告が出る。遷音速（:math:`M\approx0.8`\ 〜\ 1.2）は係数の変化が大きいので、この帯に点を置く。
* **迎角**\ ：飛翔で到達する迎角の上限まで解く。解いた最大の迎角 :math:`\alpha_e` の先は、その 2 倍（上限 90°）まで部品積み上げ法の値へなめらかにつなぐ（:eq:`eq-cfd-beyond`）。サンプルの設定（解く迎角 0、4、8°）では、8° から 16° の間でつなぎ、16° から 90° は部品積み上げ法の値になる。
* **ケース数**\ ：:math:`n_\alpha` 個の正の迎角と :math:`n_\beta` 個の正の横滑り角を指定したとき、1 つの Mach 数の ``"none"`` のケース数は :math:`(2n_\alpha+1)(2n_\beta+1)` である。対称性を使えば、解かない符号が減る（:ref:`sec-cfd-stl-symmetry`）。``roll_damping`` は Mach 数ごとに 2 ケースを足す。

ケースの並び順は、Mach 数が小さい順、各 Mach 数の中で横滑り角、その中で迎角である。
付属の ``examples/sample_cfd.toml`` は 12 ケース、``examples/sample_rans.toml`` は :math:`M=0.3,0.8,1.2,2` と :math:`\alpha=0,\pm4,\pm8^\circ` の 20 ケースである。

ロール減衰と横滑りは、必要なときだけ解く。

* ``roll_damping = true`` にすると、各 Mach 数で :math:`\alpha=\beta=0`、:math:`\hat p=\pm` ``roll_rate_hat``\ （既定 0.01）の 2 ケースを、機体が回る流れとして全領域で解く（:ref:`sec-cfd-roll-cases`）。Euler だけで使える。2 ケースの差からロール減衰微係数 :math:`C_{lp}` を求め、係数表に入れる（:ref:`sec-cfd-roll-post`）。差を取るので、フィンのカントや非対称による回転なしのずれは打ち消される。指定しないときの :math:`C_{lp}` は部品積み上げ法の値である。
* ``betas_deg`` に 0 以外を入れると横力 :math:`C_Y` とヨーモーメント :math:`C_n` が CFD から得られるが、係数表が一般表になり、ケースが増える。

.. _sec-cfd-rans-use:

RANS の使い方
~~~~~~~~~~~~~

``model = "rans"`` は、Spalart–Allmaras モデルの RANS 方程式を、壁面にプリズムの境界層を置いたメッシュで解く（式は :ref:`sec-cfd-rans-eq`）。使う前に知っておくことをまとめる。

**使える条件**\ 。次のどれかに当たると、計算を始める前にエラーになる。

* ``surface = "stl"``\ ：境界層のメッシュは作り直した形状と CAD のモデルにしか作らない。
* ``surface = "panel_mesh"`` で、フィンの配置が :math:`y=0` について対称でない（``fin_roll_deg`` が 10 度の 4 枚フィンなど）：境界層のメッシュは半領域で作り、全領域はそれを鏡映して作るからである。``surface = "step"`` では、形状が対称でなければ全領域に直接プリズム層を積むので、この制約はない。
* ``roll_damping = true``\ ：ロール減衰は Euler だけで使える。
* ``yplus`` が 5 より大きく 30 より小さい、または 150 より大きい（:ref:`sec-cfd-prism`）。

**既定値**\ 。``model = "rans"`` で次の項目を設定しないときは、Euler と異なる既定値を使う。
``wall_size`` 5 mm、``iterations`` 4000（2 次精度の段の反復数）、``min_iterations`` 400、``stable_window`` 200 である。``yplus`` の既定は 1 で、壁面まで境界層を解像する。
``z_mirror_mesh`` は効かず、``alpha_mode = "mirror"`` では負の迎角も解く（メッシュが :math:`z` について厳密に対称でないため）。底面は常に平らで、尾部フェアリングは置かない。
これらの項目を明示したときは、RANS の既定ではなくその値を使う。Euler の設定ファイルの ``model`` だけを ``"rans"`` に変えたときは、設定ファイルにある ``wall_size`` と ``iterations`` がそのまま効く。
``examples/sample_rans.toml`` は、これらを設定せず RANS の既定のままにしたサンプル機体の設定である（半モデル、:math:`M=0.3,0.8,1.2,2`\ 、:math:`\alpha=0,\pm4,\pm8^\circ` の 20 ケース、``alpha_mode = "mirror"``\ 、``yplus = 1``）。

**警告**\ 。次の場合は、計算を止めずに警告を出す。

* ``wall_size`` が 4 mm より小さい：サンプル機体の半モデル 1 ケースが 5 mm で約 7.5 GB、3 mm で約 19 GB のメモリを要する（実測から外挿した値。見積もりは余裕を足して 9 GB と 22 GB）という警告。``cfd-check --mesh`` で見積もりを確かめる。
* ``yplus`` が 30 から 150（壁関数）：検証した使い方は壁面を解像する ``yplus`` 1 であるという警告。

**抗力の内訳**\ 。軸力は、壁面（``wall``）の力の積分（圧力と摩擦）に、底面の項、追加抗力、二つの増分を足したものである（:eq:`eq-cfd-ca-rans`）。
底面の項は ``rans_base`` で選ぶ。``"model"``\ （既定）は部品積み上げ法の底面抗力、``"cfd"`` は CFD が底面に積分した力である。``"cfd"`` の底面の力は検証していない。
RANS の壁は滑らかなので、設定の粗さの効果を部品積み上げ法の摩擦の差（粗さの増分）として足し、``panel_mesh`` では鈍くしたフィンの縁の分を差し引いた設定の縁の抗力（フィンの縁の増分）を足す。部品積み上げ法の摩擦抗力そのものは足さない。
ケースごとの内訳は ``cfd_report.json`` の ``axial_breakdown`` に記録される。

**結果の状態**\ 。RANS のケースは ``converged``、``unconverged``\ （係数が一定のまま、または振動のまま窓の平均を受け入れた）、失敗のいずれかになる（:ref:`sec-cfd-int-rans-accept`）。

メッシュの大きさ
----------------

.. _sec-cfd-mesh-size:

メッシュの大きさの決め方
~~~~~~~~~~~~~~~~~~~~~~~~

体積メッシュは四面体である。セルの大きさは、壁面までの距離 :math:`s` の関数

.. math::
   :label: eq-cfd-size

   h(s)=\min\bigl(h_{\max},\;h_w+0.2\,s\bigr),\qquad h_{\max}=\frac{R_\infty}{10}

で決める。:math:`h_w` は ``wall_size``\ （壁面の要素の大きさ、Euler の既定は 2 mm）、:math:`R_\infty` は遠方境界の半径である。壁から離れるにつれ、要素の列ごとに約 1.2 倍の割合で大きくなる。
フィンの面は ``fin_size`` :math:`h_f` で細かくする。フィンの面までの距離 :math:`s_f` について

.. math::
   :label: eq-cfd-size-fin

   h(s,s_f)=\min\bigl(h_{\max},\;h_w+0.2\,s,\;h_f+0.2\,s_f\bigr)

とし、フィンの面では :math:`h_f`、そこから同じ割合で大きくなって :math:`h_w` につながる。
``fin_size = "auto"`` の :math:`h_f` は、Euler で :math:`\min(h_w,\,t/4)`\ （:math:`t` はフィンの厚さ。サンプル機体では 4 mm の 1/4 で 1 mm）、RANS で :math:`h_w` である。:math:`h_f=h_w` のときは :eq:`eq-cfd-size` だけで決まる。
フィンの面は、面の重心が胴体の外形（尾部フェアリングを含む）よりスパンの 1/10 以上外にある壁面である。ランチラグやレールボタンは胴体からの高さがスパンの 1/10 より小さければ、フィンの面とは扱わない。
曲率の大きい部分（ノーズ、フィンの縁）は、壁面の要素を細かくする。
各量の位置関係は :numref:`fig-cfd-mesh-params` に示す。

.. _fig-cfd-mesh-params:

.. figure:: /_generated/tikz/cfd_mesh_params.*
   :width: 100%

   メッシュの大きさを決める量。(a) 壁面に垂直な断面。RANS では壁のすぐ上に高さ :math:`h_1`\ （``yplus`` から決まる）で始まるプリズム層を積み、その外側の四面体が壁から離れるほど大きくなって遠方境界 :math:`R_\infty`\ （``farfield``）に達する。
   (b) フィンの断面。フィンの面は ``fin_size`` :math:`h_f`、胴体の面は ``wall_size`` :math:`h_w` の要素で覆う。
   見るところは、``fin_size`` が厚さ :math:`t` より十分細かいことと、``yplus`` が壁のすぐ上の層の高さだけを決めることである。

:numref:`fig-cfd-mesh-surface` に壁面のメッシュ、:numref:`fig-cfd-mesh-section` に対称面での体積メッシュの断面を示す。
ノーズ先端は曲率が大きいので要素が細かくなり、フィンの付け根では胴体とフィンの面が一つの立体として交わる。

.. _fig-cfd-mesh-surface:

.. figure:: /_generated/plots/cfd_mesh_surface.*
   :width: 100%

   サンプル機体の壁面のメッシュ（``wall_size`` 2 mm、フィンの面は ``fin_size = "auto"`` で 1 mm、半モデル）。どちらも :math:`+y` 側から機体の外側の面を見た図で、左はノーズ先端（先端から 9 cm。先端が右を向く）、右はフィンの付け根の前縁側。

.. _fig-cfd-mesh-section:

.. figure:: /_generated/plots/cfd_mesh_section.*
   :width: 100%

   対称面 :math:`y=0` での体積メッシュの断面。左から計算領域の全体、機体のまわり、フィンと尾部フェアリングの拡大。灰色の部分は機体（とフィン）の内部である。メッシュは :numref:`fig-cfd-mesh-surface` と同じ。

``wall_size`` と ``fin_size`` が精度、セル数、時間に効く量を、測定で示す。

**wall_size**。:math:`M=2`、半頂角 10° の円すいの表面圧力を Taylor–Maccoll の解と比べると、``wall_size`` 10、5、2.5 mm で誤差は 11.2、5.4、1.8 % と減る。
サンプル機体の :math:`C_N`\ （:math:`\alpha=4^\circ`、フィンの面は 1 mm、半モデル）は、``wall_size`` を細かくすると単調に増える。
既定の 2 mm は最も細かい 1.8 mm（156 万セル）より、:math:`M=0.8` で 0.14 %、:math:`M=2` で 0.13 % 低く、一般化した Richardson 外挿値より 0.3〜0.6 % 低い。
外挿値は当てはめに使うメッシュで 0.3 % 変わる目安である。5 mm では 1.8 mm の値より :math:`M=0.8` で 1.7 %、:math:`M=2` で 1.2 % 低い（セル数は 86 万と 142 万）。10 mm でも 77 万セルで、3〜5 % 低い。
STL のメッシュは作成のたびに少し変わるので、:math:`C_N` には、この差を超える不確かさがありうる。詳細は :ref:`sec-vaero-cfd-mesh` にある。

**fin_size**。厚さ 4 mm のフィンを ``wall_size`` の要素で覆うと、Euler の解には亜音速で :math:`\alpha=0` にも偽の法線力が残る。フィンの厚さを要素が解像できないためで、厚さの 1/4 の要素で消える（原因は :ref:`sec-cfd-val-asym-cause`）。
:numref:`tbl-cfd-fin-size` は、サンプル機体の半モデル（Euler、:math:`M=0.8`、4 ランク）で ``wall_size`` と ``fin_size`` を変えた結果である。
同じデータを :numref:`fig-cfd-fin-size-cost` に図で示す。
``fin_size`` を細かくすると :math:`\alpha=0` のずれは 0 に近づくが、セル数と時間が増える。``wall_size`` 2 mm でフィンの面を 2 mm にした場合（92 万セル）と既定（140 万セル）を :math:`M=0.8` と 2 の 4 ケースで比べると、既定の計算時間の合計は約 1.9 倍である。
RANS は、フィンの面を厚さの 1/4 にするとプリズム層の柱が増えすぎて 1 ケースのメモリに収まらない。:math:`\alpha=0` のずれも底面の後流によるもの（:math:`C_N(4^\circ)` の 1〜3 %）なので、``wall_size`` のままにする。

.. _tbl-cfd-fin-size:

.. list-table:: ``wall_size`` と ``fin_size``\ （サンプル機体、Euler、半モデル、:math:`M=0.8`、``alpha_mode = "single"``、``doc/data/cfd/fin_resolution.csv``）
   :header-rows: 1
   :widths: 16 16 16 18 16 18

   * - ``wall_size``
     - ``fin_size``
     - セル数
     - :math:`C_N(\alpha=0)`\ （真値 0）
     - :math:`\alpha=0` の 1 ケース [s]
     - 4 ランクの合計のピーク [GB]
   * - 6 mm
     - 6 mm
     - 16.6 万
     - :math:`-0.083`
     - 22
     - 0.5
   * - 6 mm
     - 1 mm
     - 81.3 万
     - :math:`-0.011`
     - 148
     - 2.0
   * - 2 mm
     - 2 mm
     - 92.1 万
     - :math:`+0.052`
     - 111
     - 2.3
   * - 2 mm
     - 1 mm（既定）
     - 139.3 万
     - :math:`+0.002`
     - 364
     - 3.3

.. _fig-cfd-fin-size-cost:

.. figure:: /_generated/plots/cfd_fin_size_cost.*
   :width: 90%

   ``fin_size`` とセル数（左）、:math:`\alpha=0` と :math:`4^\circ` の計算時間（中央）、:math:`\alpha=0` の :math:`C_N`\ （右）の関係（サンプル機体、Euler、半モデル、:math:`M=0.8`）。
   線は ``wall_size`` ごとに結ぶ。データは ``doc/data/cfd/fin_resolution.csv``。
   見るところは、``fin_size`` を細かくするとセル数と時間が増え、:math:`\alpha=0` の偽の法線力が 0 に近づくことである。

STL と CAD のモデルのフィンの縁の面も、``fin_size`` を細かくして初めて圧力抗力が軸力に入る。サンプルの STL の厚さ 4 mm のフィンは前縁と後縁が平らな面で、``wall_size`` 5 mm のままでは縁の面の圧力が出ず、:math:`M=0.8` の圧力の軸力が負になる（フィンの面を 1 mm にすると正になる。:ref:`sec-cfd-val-fin-size`）。

``max_geometry_error`` は ``stl`` の忠実度のエラーの上限である（:ref:`sec-cfd-fidelity`）。

.. _sec-cfd-prism:

境界層のプリズム層（RANS）
~~~~~~~~~~~~~~~~~~~~~~~~~~

RANS のメッシュは、壁面（``wall`` と ``base``）の近くをプリズムの層で、その外側を四面体で埋める。``yplus`` が壁のすぐ上の層の高さを決める。

最初の層の高さ :math:`h_1` は、ノーズ先端から全長の 5 % の位置で壁面の :math:`y^+` が ``yplus`` になるように、平板の乱流の摩擦の式から決める。
解く最大の Mach 数 :math:`M_{\max}` と基準高度の大気で :math:`U=M_{\max}a`、:math:`x=0.05L` として

.. math::
   :label: eq-cfd-first-height

   C_f=0.0576\,\mathit{Re}_x^{-1/5},\qquad u_\tau=U\sqrt{C_f/2},\qquad h_1=\frac{y^+\nu}{u_\tau}

である。全長の位置より境界層の薄いノーズ側で決めるので、ほかの位置の :math:`y^+` は ``yplus`` より小さくなる。
``cfd_report.json`` の ``est_wall_yplus`` は、この :math:`h_1` を全長の位置で :math:`y^+=1` になる高さで割った値で、機体の後部での :math:`y^+` の見積もりである。

層の高さは 1.2 倍ずつ増やし、最後の層が :math:`1.3\,h_w` 以上になる最小の層数 :math:`n` で止める。厚さは :math:`T=h_1(1.2^n-1)/0.2` である。
最後の層の高さがその外側の四面体の大きさとほぼ同じなので、層の上面でセルの大きさが急に変わらない。
厚さ :math:`T` には上限があり、機体の後端での平板の乱流境界層の厚さ :math:`0.37\,L\,\mathit{Re}_L^{-1/5}`\ （解く最小の Mach 数）の 1.5 倍と、胴体の最大半径の 0.5 倍の小さいほうを超えるときは、層の数を減らす。

``yplus`` の範囲は次のとおりである。

* 5 以下（既定 1）：最初の層を粘性底層に置いて壁面まで境界層を解像する。検証した使い方である。
* 30〜150：対数則に基づく壁関数を使う。検証していないので警告を出す。
* 5 より大きく 30 より小さい値、または 150 より大きい値：どちらの扱いも成り立たないので、計算を始める前にエラーにする（``IY-C123``）。

サンプル機体の半モデル（``yplus = 1``、Mach 数 0.8 と 2、``fin_roll_deg = 0``）では、どの ``wall_size`` でも層の厚さは上限で決まり、49 層、21.7 mm になる。セル数、メッシュの作成時間、メモリの見積もりは :numref:`tbl-cfd-rans-mesh` のとおりである。

.. _tbl-cfd-rans-mesh:

.. list-table:: RANS のメッシュ（サンプル機体の半モデル、``yplus = 1``、``doc/data/cfd/rans_mesh_sweep.csv``。作成時間は ``cfd-check --mesh`` の全体の時間で gmsh は 1 スレッド、メモリは :eq:`eq-cfd-mem-rans` による 4 ランクの見積もり）
   :header-rows: 1
   :widths: 14 14 14 12 16 14 16

   * - ``wall_size``
     - 節点数
     - セル数
     - 層数
     - 境界層の厚さ
     - メッシュの作成時間
     - メモリの見積もり
   * - 8 mm
     - 34.8 万
     - 87 万
     - 49
     - 21.7 mm
     - 13 s
     - 4.3 GB
   * - 6 mm
     - 54.5 万
     - 128 万
     - 49
     - 21.7 mm
     - 18 s
     - 6.7 GB
   * - 5 mm
     - 73.5 万
     - 168 万
     - 49
     - 21.7 mm
     - 22 s
     - 9.0 GB
   * - 4 mm
     - 108.4 万
     - 242 万
     - 49
     - 21.7 mm
     - 30 s
     - 13.2 GB
   * - 3.5 mm
     - 138.5 万
     - 309 万
     - 49
     - 21.7 mm
     - 36 s
     - 16.8 GB
   * - 3 mm
     - 184.5 万
     - 414 万
     - 49
     - 21.7 mm
     - 43 s
     - 22.3 GB
   * - 2.5 mm
     - 257.8 万
     - 582 万
     - 49
     - 21.7 mm
     - 59 s
     - 31.1 GB

全領域は節点数もメモリも約 2 倍になる。メッシュは、不正なプリズム、体積が 0 以下のセル、境界のマーカのない面がないことを確かめてから使い、通らないときは ``wall_size`` や ``yplus`` を変えるよう示すエラーで止まる（``mesh error: the prismatic boundary layer is invalid``）。
``surface = "step"`` では、ランチラグの穴やレールボタンのつばの下のような狭いすき間で向かい合う壁の層が交わらないように、層を薄くする。CAD の縁は鈍くしないので、鋭い縁や狭いすき間でプリズム層が作れないときは、CAD で縁を少し丸めるか、Euler を使う。
``cfd-check --mesh`` は、層数、最初の層の高さ、厚さ、不正なプリズムの数（常に 0）も表示する。

計算量と資源
--------------

.. _sec-cfd-resources:

ケース数と 1 ケースの重さ
~~~~~~~~~~~~~~~~~~~~~~~~~

ケース数は行列の大きさで決まる（:ref:`sec-cfd-choose-matrix`）。1 ケースの重さは、``wall_size`` と ``fin_size`` で決まるセル数による。
時間とメモリの測定値は :ref:`page-vv-performance` にまとめてあり、本章の数値はそこから引いたものである。
既定の 2 mm（Euler ではフィンの面は厚さの 1/4、サンプル機体では 1 mm）で、サンプル機体の半モデルは約 140 万セル、1 ケースが 4 ランクで 372〜904 s（中央値 約 520 s）である。
サンプル機体の 16 ケース（:math:`M=0.3,0.8,1.2,2.0` × :math:`\alpha=0,2,4,8^\circ`）を 4 ランクで 1 ケースずつ解くと約 2.5 時間かかる（:ref:`sec-vaero-timing`）。
RANS は、5 mm（73.5 万節点）の 1 ケースが暖機起動で約 15〜30 分、冷えた起動で 30〜60 分、サンプル機体の 20 ケースが 4 ランク、2 ケースずつ同時で約 4.4 時間である（:ref:`sec-cfd-rans-cost`）。

1 ケースのメモリは、ランク数 :math:`n_r`、セル数 :math:`N` に対して

.. math::
   :label: eq-cfd-mem

   B \approx 40\ \mathrm{MB}\times n_r+3.3\ \mathrm{kB}\times N

と見積もる（Euler）。RANS は、節点数 :math:`N_n` から

.. math::
   :label: eq-cfd-mem-rans

   B \approx 40\ \mathrm{MB}\times n_r+\max\bigl(12\ \mathrm{kB}\times N_n,\ 3.3\ \mathrm{kB}\times N\bigr)

と見積もる。同じメッシュの Euler の見積もりを下回ることはない。
実測との比較の例として、Euler の 142 万セルは見積もりが 4.8 GB で、実測の最大ランクのピークの 4 倍は 4.6 GB であった。
RANS の ``wall_size`` 4.8 mm（78.7 万節点）の半モデルは、実測のピークがランクの合計で 7804 MiB であった（``doc/data/cfd/rans_memory.csv``）。節点あたり 12 kB の根拠は :ref:`sec-vperf-rans-memory` にある。
式の求め方と測定の詳細は :ref:`page-vv-performance` にある。

.. _sec-cfd-parallel:

ranks_per_case と parallel_cases の決め方
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

``ranks_per_case`` は 1 ケースの MPI ランク数、``parallel_cases`` は同時に動かすケース数（どちらも既定 4）で、積は使えるスレッド数以内でなければならない。
ランクを増やすと 1 ケースが速くなる。ただし、メッシュを分割するランクの間の通信が増えるので、スレッドあたりの効率は下がる。
同時ケースを増やすと全体の処理量は上がるが、メモリがその分だけ要る。
ランクを増やしたときのメモリの増えは小さい（1 ランクあたり 40 MB）ので、メモリが足りないときは同時ケースを減らす。

:numref:`tbl-cfd-parallel` は、使えるスレッドが 16 のときの組合せを、式 :eq:`eq-cfd-mem` と :eq:`eq-cfd-mem-rans` による見積もりで示す。
メモリの合計は、同時に動かすケース数だけ 1 ケースのメモリが要るとして求めた。

.. _tbl-cfd-parallel:

.. list-table:: ランク数と同時ケース数の組合せ（スレッド 16 を使い切る場合のメモリの見積もり。``doc/data/cfd/resource_plan.csv``）
   :header-rows: 1
   :widths: 14 14 18 18 18 18

   * - ``ranks_per_case``
     - ``parallel_cases``
     - Euler（142 万セル）1 ケース [GB]
     - Euler の合計 [GB]
     - RANS（5 mm、73.5 万節点）1 ケース [GB]
     - RANS の合計 [GB]
   * - 1
     - 16
     - 4.7
     - 75.6
     - 8.9
     - 141.8
   * - 2
     - 8
     - 4.8
     - 38.1
     - 8.9
     - 71.2
   * - 4（既定）
     - 4
     - 4.9
     - 19.4
     - 9.0
     - 35.9
   * - 8
     - 2
     - 5.0
     - 10.0
     - 9.1
     - 18.3
   * - 16
     - 1
     - 5.3
     - 5.3
     - 9.5
     - 9.5

選ぶ手順は次のとおりである。

#. ``[resources]`` の ``threads`` に、使えるスレッド数を書く（0 なら自動）。
#. 表の合計が使えるメモリ以内の行のうち、``ranks_per_case`` が小さい行を選ぶ。ランクが小さいほどスレッドあたりの効率がよく、同時ケースのほうが全体は速く済むからである。
#. すべての行が収まらないときは、``wall_size`` を粗くするか、ケース行列を減らす。

どちらも 0 にすると自動になり、ランク数は 4 とスレッド数の小さいほう、同時ケース数は ``スレッド数 / ランク数`` になる。積が使えるスレッド数を超えるときは、まず同時ケース数を、次にランク数を減らす。

``[resources]`` の ``threads`` はスレッド数の上限で、``memory_gb`` は同時に動かすケースのメモリの予算である。
``memory_gb = 0``\ （既定）のときの予算は、使えるメモリ（空きメモリと、コンテナなどの上限の小さいほう）で、正の値を指定するとそれを使う。
予算に収まるまで同時ケース数を減らし、1 ケースの見積もりが予算を超えるときは、メッシュを作ったあと、計算の前にエラーで止まる。
``ignisyeet cfd-check 設定ファイル --mesh`` は、解決した並列度と 1 ケースの見積もりを表示する。


ケースの状態と収束
--------------------

.. _sec-cfd-int-convergence:

Euler のケースの状態
~~~~~~~~~~~~~~~~~~~~

各ケースは、次の四つの状態のどれかになる。状態は ``cfd_cases.csv`` の ``status`` に記録され、収束しなかったケースと失敗したケースの一覧は実行の最後の警告とエラーに出る。

.. list-table:: Euler のケースの状態
   :header-rows: 1
   :widths: 16 44 40

   * - 状態
     - 意味
     - 係数の扱い
   * - 収束（``converged``）
     - 残差が ``convergence`` の目標に達し、係数も落ち着いた
     - 最後の反復の係数
   * - 早期終了（``accepted``）
     - 残差の目標には達しないが、残差が十分下がり、係数が落ち着いたので止めた
     - 直近の反復の平均
   * - 受け入れ（未収束）
     - 反復の上限か時間の上限（``timeout_minutes``）に達したが、最後の窓で力とモーメントがほぼ一定だった
     - 窓の平均。警告に載る
   * - 失敗
     - SU2 のエラー、発散、または収束も安定もしていない
     - 使わない

残差が目標に達しても係数がゆっくり変化していることがある（サンプル機体の :math:`M=0.8` で、残差が :math:`10^{-6}` に達した時点の :math:`C_N` は収束した値より 0.56 % 低かった）ので、残差だけでは止めず、係数の落ち着きも見る。
既定の ``min_iterations`` と ``stable_window`` は、早期終了なしの基準との係数の差が 0.06 % 以内で、4 ケースの時間を 34 % 減らす値である（:ref:`sec-cfd-early-stop`）。

失敗したケースは、表の作成で飛ばして補間する（最小または最大の Mach 数の列の失敗は補えず、エラーになる）。
平らな底面の Euler（``base_treatment = "flat"``）は収束しないことがあり、そのときも失敗として扱う。
収束しないケースや失敗したケースが出たときは、``base_treatment`` と ``wall_size`` を見直し、``iterations`` を増やして再実行する。
収束の履歴の例は :ref:`page-howto-cfd-table` に示す。

.. _sec-cfd-int-rans-stages:

RANS の起動と発散
~~~~~~~~~~~~~~~~~

RANS のケースは、各 Mach 数の :math:`\alpha=\beta=0` を自由流から（冷えた起動）、1 次精度で解いてから 2 次精度で解く。
迎角と横滑り角の列は、隣のケースの解から続けて 2 次精度で解く（暖機起動）。残差の大きさは Mach 数で変わるので、Mach 数をまたいで暖機起動しない。
暖機起動のケースがうまく収束しないときは、自由流から解き直す。
発散したケースは、反復の上限を待たずに止め、安定に寄せた設定で一度だけ解き直す。それでも発散すれば失敗とする。
発散や解き直しは警告として表示される。失敗したケースが続くときは ``cfl`` を下げて再実行する（:ref:`cfg-aero-cfd-cfl`）。

.. _sec-cfd-int-rans-accept:

RANS のケースの状態
~~~~~~~~~~~~~~~~~~~

RANS のケースの状態は、``cfd_cases.csv`` の ``rans_verdict`` で確かめる。判定は、冷えた起動の残差の最大値からの下がりと、係数の平らさで決まる。

.. list-table:: RANS のケースの状態
   :header-rows: 1
   :widths: 16 44 40

   * - 状態
     - 意味
     - 係数の扱い
   * - 収束（``converged``）
     - 残差が十分に下がり、係数が平らでこれから先の変化も小さい
     - 最後の反復の係数
   * - 停滞（``stalled``、未収束）
     - 残差は下がりきらずに止まったが、係数は平らである
     - 二つの窓の平均。警告に載る
   * - 振動（``limit_cycle``、未収束）
     - 残差は十分下がり、係数は小さな幅で振動している
     - 窓の平均。警告に載る
   * - 失敗
     - どれでもない
     - 使わない。残差の下がりと係数の状態が理由として示される

停滞と振動は ``unconverged`` として扱われ、実行の最後の警告に載る。収束と停滞の判定は、遅い変化を見逃さないように、係数の傾きから残る変化を見込む条件を含む。
NASA TMR の平板の SU2 の履歴にこの判定を当てはめると、止めた反復の抗力は残差を 8 桁下げた値と最大 0.3 % しか違わない（6 つの格子と Mach 数の組。``doc/data/cfd/rans_flatplate.csv``）。
停滞や振動のケースの係数は窓の平均であり、収束したケースより不確かさが大きい。使う前に、その Mach 数と迎角のケースが飛翔の主な範囲にあるかを確かめる。
受け入れたケースの壁面の :math:`y^+` が ``yplus`` の目標を超えるときは警告が出る。サンプル機体の検証では 0.4〜0.7 であった。

適用範囲と限界
----------------

* **解く内容**\ ：定常解を (M, α, β) の点ごとに並べた準定常の表である。角速度に依存する非定常な空力を、すべて表現するものではない。ピッチ・ヨー減衰は CFD が直接与えるものではなく、交差する微係数は扱わない（:ref:`sec-cfd-trust`）。
* **Euler**\ ：粘性による摩擦とはく離を含まない。平らな底面は解が定まらないので、``panel_mesh`` では尾部フェアリング、``stl`` と ``step`` では ``base_treatment`` で扱う。底面抗力は経験式による。
* **RANS**\ ：``panel_mesh``\ （フィンの配置が :math:`y=0` について対称なときだけ）と ``step`` で使える。1 ケースのメモリが大きい。検証した範囲は :numref:`tbl-rans-gates` にある。
* **一般表**\ ：解いた迎角と横滑り角の外側はモデルの値で、非対称な形状ではあてにならない（:ref:`sec-cfd-region-outside`）。
* **形状**\ ：``panel_mesh`` は回転体と台形フィンに限られる。``stl`` と ``step`` は、フィンの縁の面を ``fin_size`` で解像した場合に限り、縁の圧力抗力が軸力に入る。
* **メッシュ**\ ：メッシュ収束は完全でなく、フィンの面を 1 mm より細かくした計算はない。STL のメッシュは作成のたびに少し変わる。

実装上の制約の一覧は :ref:`sec-lim-cfd` にある。

精度の証拠
------------

.. _sec-cfd-trust:

どこまで信頼してよいか
~~~~~~~~~~~~~~~~~~~~~~

CFD が与える値と、モデルで補う値を分けて理解する（詳細は :ref:`sec-cfd-supplied`）。

* CFD の解から得る値：法線力 :math:`C_N`、圧力中心 :math:`x_{cp}`、軸力のうち圧力による抗力（RANS では摩擦も）、横滑りを解けば横力 :math:`C_Y` とヨーモーメント :math:`C_n`、``roll_damping`` ならロール減衰 :math:`C_{lp}`。
* モデルで補う値：Euler の摩擦、底面、フィン縁、ボートテールの抗力、ピッチ・ヨー減衰 :math:`C_{mq}`\ 、:math:`C_{nr}`\ 、ロールのカント強制 :math:`C_{l\delta}`、解いた迎角と横滑り角の外側。

検証と実測との比較を、スコープを区別して示す。

* **検証**\ （解析解、相関式、収束との比較）：
  円すいの表面圧力は、メッシュを細かくすると Taylor–Maccoll の解に近づく（:ref:`sec-vaero-cfd`）。
  :math:`z` 対称メッシュと ``alpha_mode = "mirror"`` で :math:`\alpha=0` の見かけの非対称性が消える。ロール減衰と横滑りの符号は軸対称表を展開した値と一致する（:ref:`sec-cfd-val-roll`）。
  RANS は、平板の摩擦が NASA TMR の SA の解と 0.8 % 以内、:math:`M=2` の van Driest II と 1.2 % 以内（最も細かい格子）で一致し、円すいとフィンなし胴体の摩擦の軸力も平板の見積もりと 2.5〜7.4 % で一致する。
  サンプル機体の RANS の :math:`C_N`\ 、:math:`C_A`\ 、:math:`x_{cp}`\ （:math:`M=0.8,2`、:math:`\alpha=4^\circ`）は、メッシュに対して単調に収束する（GCI は :math:`C_N` で 0.5 % 以内、:math:`C_A` で 1.9 % 以内）。
* **相互比較**\ （手法どうし）：
  亜音速の :math:`C_{N\alpha}` は、CFD とパネル法の差が 2.5 %（:math:`M=0.8`）から 9 %（:math:`M=0.3`）で、部品積み上げ法より CFD に近い（:ref:`sec-vaero-compare`）。
  STL をそのまま解く計算は、作り直した形状による計算と相互比較した（:ref:`sec-cfd-val-stl`）。
  RANS は Euler の CFD と :math:`C_N` で 3.6〜5.1 %、:math:`x_{cp}` で 4 mm 以内で一致する（形状の違いを含む。:ref:`sec-cfd-rans-compare`）。
* **妥当性確認**\ （実測との比較）：
  Basic Finner の超音速（Mach 1.08–4.30）の射場試験との比較だけで、Euler の :math:`C_{N\alpha}` は実測より 3〜11 % 小さく、:math:`x_{cp}` は 0.2 口径以内で合う（:ref:`page-vv-validation-basic-finner`）。

メッシュ収束（:ref:`sec-vaero-cfd-mesh`）は、フィンの面を 1 mm にしたまま ``wall_size`` を変えると :math:`C_N` が単調に増え、既定の 2 mm は最も細かい 1.8 mm より 0.1 %、外挿値より 0.3〜0.6 % 低い。
RANS の検証の項目と結果は :numref:`tbl-rans-gates`\ （:ref:`sec-cfd-rans`）にあり、次の条件は確かめていない、または基準を満たさない。

* 亜音速の :math:`\alpha=0` の非対称：:math:`M=0.8` で :math:`C_N(0)` が 0.011〜0.034 残る（底面の後ろの剥離した流れの非対称な解）。``alpha_mode = "mirror"`` が :math:`\pm\alpha` の組合せで取り除く。
* 低い亜音速：:math:`M=0.3` の :math:`\alpha=0,-4^\circ,-8^\circ` は残差が周期的に振動し、収束しない。サンプル機体の係数表（20 ケース）で収束と判定されたのは 15 ケース（75 %）で、基準の 90 % に届かない。
* 底面の流れ：底面の圧力（``rans_base = "cfd"``）は確かめていない。
* CAD のモデル：``surface = "step"`` の RANS はメッシュの検査までを確かめ、SU2 の解は確かめていない。

検証の全体は :ref:`page-vv-cfd-verification` と :ref:`page-vv-rans-verification`、手法どうしの相互比較は :ref:`page-vv-cross-comparison`、計算時間とメモリの測定は :ref:`page-vv-performance` にある。

関連する設定キー
------------------

値の完全な定義は :ref:`page-reference-config-cfd` にある。

.. list-table::
   :header-rows: 1
   :widths: 22 22 56

   * - キー
     - 既定値
     - 効果の方向
   * - :ref:`model <cfg-aero-cfd-model>`
     - ``"euler"``
     - ``"rans"`` で摩擦を CFD から得るが、1 ケースのメモリと時間が大きい
   * - :ref:`surface <cfg-aero-cfd-surface>`
     - ``"panel_mesh"``
     - ``"stl"``、``"step"`` で付属物を含める。係数表が一般表になる
   * - :ref:`base_treatment <cfg-aero-cfd-base_treatment>`
     - なし（``stl`` と ``step`` の Euler で必須）
     - ``"fairing"`` は形状を変えるが収束する。``"flat"`` は収束しないことがある
   * - :ref:`tail_fairing <cfg-aero-cfd-tail_fairing>`
     - 6（底面半径）
     - 0 で平らな底面（``panel_mesh``）
   * - :ref:`rans_base <cfg-aero-cfd-rans_base>`
     - ``"model"``
     - ``"cfd"`` は底面の力を CFD から得るが、検証していない
   * - :ref:`shape_symmetry <cfg-aero-cfd-shape_symmetry>`
     - ``"auto"``
     - 検査に通った対称性だけを使う。``"none"`` はケースとセルが増える
   * - :ref:`symmetry <cfg-aero-cfd-symmetry>`
     - ``true``
     - ``false`` で全領域を強制し、セルが約 2 倍になる
   * - :ref:`z_mirror_mesh <cfg-aero-cfd-z_mirror_mesh>`
     - ``true``
     - :math:`\alpha=0` の見かけの非対称を消し、負の迎角を解かずに済む
   * - :ref:`farfield <cfg-aero-cfd-farfield>`
     - 20（全長）
     - 10〜40 で :math:`C_N` はほぼ変わらない
   * - :ref:`alpha_mode <cfg-aero-cfd-alpha_mode>`
     - ``"mirror"``
     - :math:`\alpha=0` のずれの扱い。``"single"`` は何も引かない
   * - :ref:`machs <cfg-aero-cfd-machs>`、:ref:`alphas_deg <cfg-aero-cfd-alphas_deg>`
     - 10 点、6 点
     - 点を増やすとケース数が増える
   * - :ref:`betas_deg <cfg-aero-cfd-betas_deg>`
     - ``[0]``
     - 0 以外で一般表になり、ケースが増える
   * - :ref:`roll_damping <cfg-aero-cfd-roll_damping>`
     - ``false``
     - ``true`` で :math:`C_{lp}` を CFD から得る（Euler のみ）。Mach 数ごとに 2 ケース増える
   * - :ref:`wall_size <cfg-aero-cfd-wall_size>`
     - 2 mm（RANS は 5 mm）
     - 細かくすると精度が上がり、セル数、時間、メモリが増える
   * - :ref:`fin_size <cfg-aero-cfd-fin_size>`
     - ``"auto"``
     - 細かくすると :math:`\alpha=0` の偽の法線力が小さくなり、セル数と時間が増える
   * - :ref:`yplus <cfg-aero-cfd-yplus>`
     - 1
     - 5 以下は壁面を解像する。30〜150 は壁関数（未検証）
   * - :ref:`iterations <cfg-aero-cfd-iterations>`
     - 3000（RANS は 4000）
     - 反復の上限。上限に達すると ``unconverged`` か失敗
   * - :ref:`min_iterations <cfg-aero-cfd-min_iterations>`、:ref:`stable_window <cfg-aero-cfd-stable_window>`
     - 100 と 60（RANS は 400 と 200）
     - 早期終了の最小反復数と窓
   * - :ref:`convergence <cfg-aero-cfd-convergence>`
     - :math:`10^{-6}`
     - Euler の残差の目標
   * - :ref:`ranks_per_case <cfg-aero-cfd-ranks_per_case>`、:ref:`parallel_cases <cfg-aero-cfd-parallel_cases>`
     - 4、4
     - 1 ケースの速さと同時に解くケース数。積は使えるスレッド数以内
   * - :ref:`memory_gb <cfg-resources-memory_gb>`
     - 0（使えるメモリ）
     - 同時ケース数の上限を決めるメモリの予算
