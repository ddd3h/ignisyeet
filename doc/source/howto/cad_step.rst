.. _page-howto-cad-step:
.. _sec-cfd-step:

CAD のモデル（STEP）から係数表を作る
======================================

CAD のモデル（STEP、IGES、BREP）の立体をそのまま CFD の壁面にして、係数表を作る手順を示す（``surface = "step"``）。
ランチラグ、レールボタン、カメラのポッド、ノズルのように STL の抽出では扱えない細部も、CAD にあれば壁面に入る。
曲面は三角形に近似せず、厳密な曲面のままメッシュにする。manifold3d は要らない。
このページを終えると、CAD のモデルを STL と照合し、メッシュの報告を読み、係数表を作れる。

前提
------

* CFD のツールが入っている（:ref:`page-howto-cfd-setup`）。
* ``rocket.stl`` が機体の STL として設定済みである。飛翔計算の形状（基準面積、全長、フィンの寸法、部品積み上げ法の項）は、どの壁面のモードでも ``rocket.stl`` から抽出する。
  CAD のモデルは、この STL と同じ機体でなければならない。
* 付属の例は ``examples/sample_step.toml`` で、サンプル機体にランチラグ 1 本とレールボタン 2 個を足した ``examples/sample_rocket.step``\ （mm 単位）を解く。

手順
------

1. CAD から書き出す
~~~~~~~~~~~~~~~~~~~~

次の点を守って書き出す。

* **形式**： STEP（``.step``、``.stp``）、IGES（``.iges``、``.igs``）、BREP（``.brep``、``.brp``）。拡張子の大文字と小文字は区別しない。
* **単位**： STEP と IGES は、ファイルが宣言する長さの単位から m に換算して読む。BREP は単位を持たないので m として読む。
  宣言が誤っているときや BREP が m でないときは、``step_unit``\ （``"m"``、``"cm"``、``"mm"``、``"in"``、``"ft"``）で指定する。単位を宣言していない STEP と IGES では ``step_unit`` を使えない。単位を宣言して書き出し直す。
  STL の単位（``rocket.stl_scale``）とは別に扱うので、STL が mm で STEP が inch でもよい。
* **一つの閉じた立体**： 機体を一つのソリッドとして書き出す。複数のソリッドは一つに融合する。離れたまま残ったソリッドがあれば、すべてを壁面にして警告を出す。
  ソリッドのない面の集まりは縫い合わせて立体にするが、閉じた立体にならなければエラーになる。
* **座標系**： ``rocket.stl`` と同じ座標系（同じ原点、同じノーズの向き、同じロールの向き）にする。``rocket.nose_direction`` は STL と CAD の両方に効く。
  CAD の中でモデルを動かしたり回したりしたあとで STL だけを書き出し直すと、手順 3 の照合で止まる。
* **含めるもの**： 流れに触れる外形（ノーズ、胴体、フィン、ランチラグ、レールボタン、ポッド、底面から突き出たノズルなど）。モーターや内部の構造は含めない。
  機体の後端は、STL の後端 :math:`x=L` にある平らな後ろ向きの面（底面）で閉じる。底面より後ろの面（突き出たノズル）は底面の扱いになり、力に入らない。
* **鋭い縁**： Euler では鋭くてもよい。RANS では、鋭い縁や狭いすき間（ランチラグの穴、レールボタンのつばと胴体の間）にプリズム層が沿えないことがあり、そのときは縁を少し丸めて書き出し直す。

期待する結果： STL と同じ座標系の、閉じた一つの立体のファイルができる。

2. 設定に書く
~~~~~~~~~~~~~~~

``examples/sample_step.toml`` の ``[aero.cfd]`` は次の項目を使う。

.. code-block:: toml

   [aero]
   method = "cfd"

   [aero.cfd]
   surface = "step"
   step_file = "sample_rocket.step"   # 設定ファイルのあるディレクトリからの相対パス
   step_unit = "auto"                 # ファイルの宣言に従う
   base_treatment = "fairing"         # Euler では必須: "fairing" か "flat"
   model = "euler"

.. list-table::
   :header-rows: 1
   :widths: 24 76

   * - 項目
     - 内容
   * - ``step_file``
     - CAD のファイル。設定を読むときに、ファイルがあることと拡張子を確かめる
   * - ``step_unit``
     - 長さの単位。``"auto"``\ （既定）は、ファイルの宣言（BREP は m）に従う
   * - ``base_treatment``
     - Euler では必須。``"fairing"`` は平らな底面の後ろに円すいを付け（形が変わる）、``"flat"`` は平らな底面を残す。RANS では使わない（:ref:`cfg-aero-cfd-base_treatment`）
   * - ``wall_size`` ``fin_size``
     - 壁面とフィンの面の要素の大きさ。手順 3 の許容も ``wall_size`` で決まる
   * - ``shape_symmetry`` ``symmetry`` ``z_mirror_mesh``
     - 形状の検査に通った平面だけを使う。付属の ``sample_rocket.step`` は対称でなく、全領域を解く

``step_file`` と ``step_unit`` を ``surface = "step"`` 以外で書くとエラーになる（:ref:`IY-C120`）。
CAD のファイルの内容を書き換えたときは、係数表を作り直す。
各項目の値域は :ref:`page-reference-config-cfd` にある。``fairing`` と ``flat`` の選び方は :ref:`page-theory-cfd-choose` にある。

期待する結果： ``aero`` などの設定の要約の ``cfd`` の行に ``surface step (sample_rocket.step)`` のようにファイル名が出る。

.. _sec-cfd-step-report:

3. 読み込みと位置合わせを確かめる
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

.. code-block:: console

   $ ignisyeet cfd-check examples/sample_step.toml --mesh

メッシュを作ったあとに、読み込みと照合の報告が表示される。``examples/sample_step.toml`` の出力は次のとおりである（20 スレッドのマシンでメッシュの作成に 101 秒かかった。長い行は省略した）。

.. code-block:: text

   Mesh …/examples/out_step/cfd/mesh/mesh.su2 (built): 560092 nodes, 2690207 cells, 479830 wall triangles, 101.0s
     cell types: {"Tetrahedron 4": 2690207}
     quality (SICN, 1 = ideal): min 0.071, mean 0.783, 0 cells below 0.05
   CAD surface (surface = "step", base treatment: fairing):
     file        examples/sample_rocket.step (step, unit declared 0.001 m, step_unit auto, scale 1)
     shape       1 solid(s) in the file, 1 after fusion; 44 faces (1 base), area 0.56293 m^2, volume 0.010940 m^3
     details     3 STEP-only region(s), 1.308e-3 m^2; smallest face width 7.160e-4 m -> smallest mesh size 2.500e-4 m
     domain      full: shape symmetry none (requested auto), full mesh: auto: not symmetric (…)
     base        fairing {…}
     registration  STL -> STEP surface: max 1.193e-2 m at (1.310, 0.043, -0.045), p99 2.024e-3 m, rms 5.922e-4 m (82827 samples, allowed 1.500e-2 m; 0.6 % of the samples buried inside the STEP solid)
     STEP -> STL: max 6.198e-3 m, p99 4.211e-3 m; 3 STEP-only region(s) farther than 3.000e-3 m from the STL, 1.308e-3 m^2
     bounding box [m]: STL  x -0.0000..1.5000 y -0.1075..0.1075 z -0.1075..0.1075
                       STEP x 0.0000..1.5000 y -0.1075..0.1075 z -0.1075..0.1075
     offsets STEP - STL [m]: min (+0.0000, -0.0000, -0.0000), max (+0.0000, +0.0000, +0.0000)

各行の読み方は次のとおりである。

.. list-table::
   :header-rows: 1
   :widths: 24 76

   * - 行
     - 読み方
   * - ``file``
     - 単位の宣言と換算の倍率。倍率が想定と違えば ``step_unit`` を見直す
   * - ``shape``
     - 融合の前後のソリッドの数と面の数、面積、体積。融合後に 1 でなければ、離れたソリッドが残っている
   * - ``details``
     - CAD だけにある細部の領域の数と面積、最も細い面の幅。上の例の 3 領域は、ランチラグと 2 個のレールボタンである
   * - ``domain``
     - 計算領域と対称性の判定。上の例は鏡映対称でないので、全領域を解く
   * - ``base``
     - 底面の処理
   * - ``registration``
     - STL と CAD の距離。``max`` が ``allowed`` 以内なら通る
   * - ``bounding box`` ``offsets``
     - 包含箱の比較。``offsets`` がすべて 0 に近ければ、位置と向きがそろっている

照合は、包含箱、ノーズの先端、STL の面と CAD の面の距離を比べる。許容は ``allowed`` に表示される。許容より小さい形の違い（フィンの厚さの違いなど）は通る。通らないときの条件は :ref:`IY-F071` にある。

縫い合わせや形状の修復をしたときは ``note:`` の行に、離れたソリッドや ``"fairing"`` で形を変えたことは ``warning[IY-F5nn]:`` の行に出る。
同じ内容は、手順 4 のあとの ``cfd_report.json`` の ``step`` に入る。

期待する結果： ``registration`` の ``max`` が ``allowed`` 以内で、エラーなしで終わる。

4. 係数表を作る
~~~~~~~~~~~~~~~~

.. code-block:: console

   $ ignisyeet aero examples/sample_step.toml

CFD の実行、中断と再開、``cfd_cases.csv`` の読み方は :ref:`page-howto-cfd-table` と同じである。
CAD のモデルの係数表は、常に 6 成分の一般表になる。

* 付属の ``sample_rocket.step`` は鏡映対称でないので、全領域を解く。全領域は半領域の約 2 倍のセルで、負の迎角も解くので、``sample_cfd.toml`` の約 1.9 時間よりかなり長くかかる。
  ``wall_size`` 6 mm（フィンの面 1 mm）では、``sample_rocket.step`` の全領域は 162 万セル、同じ大きさの ``panel_mesh`` の半モデルは 81 万セルだった（``doc/data/cfd/fin_resolution.csv``）。
* 細い面（ランチラグの管の壁など）には、その幅の 1/4（``wall_size`` の 1/8 を下限とする）まで細かい要素を置くので、細部の多いモデルではセルが増える。
* RANS（``model = "rans"``）でも使える。プリズム層を CAD の壁面に積み、層の検査に通ったときだけメッシュとして使う。
  CAD の縁は鈍くしない。RANS は 1/4 領域を使わない。STEP の壁面の RANS の解は検証していないので、結果を使う前に :ref:`page-vv-rans-verification` を読む。

期待する結果： 出力先に ``aero_table.csv``、``aero_table.json``、``cfd_cases.csv``、``cfd_report.json`` が書かれる。

確認
------

* 手順 3 の報告で、``registration`` が通っている。
* ``cfd_cases.csv`` の ``status`` に ``failed`` がない（:ref:`page-howto-cfd-table`）。
* 飛翔計算の部品積み上げ法の項は ``rocket.stl`` から求める。CAD の細部の圧力による抗力は CFD の軸力に入るので、``rocket.extra_cd`` で二重に数えない（付属の例は 0）。
  Euler の摩擦抗力は STL の形状から求めるので、細部の摩擦抗力は入らない。

うまくいかないとき
--------------------

.. list-table::
   :header-rows: 1
   :widths: 46 54

   * - 症状
     - 原因と対処
   * - ``aero.cfd.surface = "step" needs step_file = …``
     - ``step_file`` を指定する（:ref:`IY-C117`）
   * - ``aero.cfd.step_file = "…": not a CAD file OpenCASCADE reads``、``no such file``
     - 拡張子を STEP、IGES、BREP のいずれかにする。パスは設定ファイルのディレクトリからの相対パスである（:ref:`IY-C118`、:ref:`IY-C048`）
   * - ``aero.cfd.surface = "step" with model = "euler" needs base_treatment``
     - ``base_treatment`` に ``"fairing"`` か ``"flat"`` を指定する（:ref:`IY-C119`）
   * - ``aero.cfd.step_unit = "…": the length unit of … cannot be read from the file``
     - 単位を宣言していない STEP か IGES である。単位を宣言して書き出し直し、``step_unit = "auto"`` にする（:ref:`IY-F070`）
   * - ``aero.cfd.surface = "step": … does not describe the same rocket as rocket.stl``
     - 照合に通らない。続く理由の行（``the nose tips differ``、``the STL extends beyond the STEP``、``the STEP is … times as large as the STL``、``the STL lies up to … m from the STEP surface``）と包含箱を見る。
       ``likely cause: units`` は単位（``step_unit`` と ``rocket.stl_scale``）、``a different orientation or position`` は向きか原点（STL と同じ座標系で書き出し直す）、``a different revision of the CAD model`` は別の版のモデルである（:ref:`IY-F071`）
   * - ``mesh error: … contains no closed solid``
     - 面が閉じた立体にならない。CAD でソリッドとして書き出す（:ref:`IY-F040`）
   * - ``mesh error: gmsh cannot mesh the surface of …``、``… duplicated or degenerate triangles``
     - CAD のモデルを直して書き出し直すか、``wall_size`` を変える（:ref:`IY-F041`、:ref:`IY-F044`）
   * - ``aero.cfd.surface = "step": the shape has no base faces …``
     - 機体の後端を平らな面で閉じる（:ref:`IY-F075`）。``"fairing"`` で平らな底面が足りないときは、底面を平らにするか ``"flat"`` にする（:ref:`IY-F074`）
   * - ``shape_symmetry = "axisymmetric" needs a surface that is mirror symmetric …``
     - 明示した対称性が検査に通らない。``"auto"`` か ``"none"`` にする（:ref:`IY-F076`、:ref:`IY-F077`）
   * - ``mesh error: the prismatic boundary layer is invalid: …``\ （RANS）
     - プリズム層が鋭い縁に沿えない。``model = "euler"`` にするか、CAD でフィンの前縁と後縁、ランチラグやレールボタンの縁を少し丸める（:ref:`IY-F038`）
   * - ``mesh error: the tetrahedral mesh outside the boundary layer failed``\ （RANS）
     - 狭いすき間（ランチラグの穴、レールボタンと胴体の間）で層の上面が交わった。すき間を広げるか埋める、縁を丸める、または Euler にする（:ref:`IY-F036`）
   * - ``warning[IY-F512]: the CAD file holds … disjoint solids``
     - 離れたソリッドもすべて壁面になる。意図したものか確かめる（:ref:`IY-F512`）
   * - ``warning[IY-F511]: base_treatment = fairing: the CAD shape is MODIFIED``
     - 円すいを付けて形を変えたことの通知である。形を変えたくなければ ``"flat"`` を選ぶ（収束しにくい。:ref:`IY-F511`）

関連
------

* 設定キー：:ref:`page-reference-config-cfd`
* 係数表を作って確かめる：:ref:`page-howto-cfd-table`
* CFD のツール：:ref:`page-howto-cfd-setup`
* 壁面の形状（panel_mesh、stl、step）の選び方、底面の処理、対称性：:ref:`page-theory-cfd-choose`
* CFD の出力ファイル：:ref:`page-reference-cfd-output-files`
