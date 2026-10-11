CFD による空力表の作成
======================

部品積み上げ法とパネル法は数秒で係数表を作るが、遷音速の抗力や複雑な形状の細部は CFD でなければ捉えられない（方法の選択は :ref:`page-theory-aero-choose`）。
``aero.method = "cfd"`` を指定すると、SU2 が機体まわりの流れを解き、その結果が係数表になる。CFD は 1 ケースごとの計算が重く、設定によって結果も時間も大きく変わる。
この章は、ツールの用意、計算の実行と結果の確かめ方、CAD のモデルの利用、出力ファイルを述べ、最後の節で係数表そのものの形式を示す。外部の風洞試験の表を使うときも、この形式で用意する。
方程式と離散化の根拠は :ref:`page-theory-cfd-equations` にあり、設定キーの値域は :ref:`page-reference-config-cfd` にある。

.. _page-howto-cfd-setup:

CFD のツールを用意する
----------------------

CFD で係数表を作るには、流れを解く SU2_CFD、体積メッシュを作る gmsh、複数ランクで 1 ケースを解くための Open MPI が要る。
``surface = "stl"`` のときは、さらに manifold3d が要る。
部品積み上げ法とパネル法だけを使うときは、この節は不要である。

.. list-table::
   :header-rows: 1
   :widths: 28 72

   * - ツール
     - 用途
   * - SU2_CFD（8.3.0）
     - 流れの解法（Euler 方程式と RANS 方程式） :cite:`su2`
   * - gmsh（Python API。4.15.2）
     - 体積メッシュの生成と、CAD のファイルの読み込み :cite:`gmsh,occt`
   * - Open MPI（``mpirun``。5）
     - ``ranks_per_case`` が 2 以上のときに必要 :cite:`openmpi`
   * - manifold3d（Python。3.5.4）
     - ``surface = "stl"`` のときだけ必要 :cite:`manifold`

SU2 は 8.3.0 に固定されている。他の版でも動くが、結果が変わりうるので警告が出る。
対象は Linux（x86_64 と aarch64）と macOS で、ネットワークへの接続と約 3 GB のディスクの空きが要る。

インストーラに ``--cfd`` を付けると、これらがまとめて入る。すでに ``ignisyeet`` を入れてあるときも、同じコマンドで追加できる。

.. code-block:: console

   $ curl -fsSL https://raw.githubusercontent.com/ddd3h/ignisyeet/main/install.sh | bash -s -- --cfd

最後に、設定ファイルの ``[aero.cfd] prefix`` に書く値（例：``"~/.local/share/micromamba/envs/ignisyeet-cfd"``）か、環境変数 ``IGNISYEET_CFD_PREFIX`` に入れる値が表示される。
``prefix`` が空のときは ``IGNISYEET_CFD_PREFIX`` が使われ、それも空のときは ``PATH`` から探される。先頭の ``~`` はホームディレクトリに展開される。

ツールが使えることは ``cfd-check`` で確かめる。

.. code-block:: console

   $ ignisyeet cfd-check examples/sample_cfd.toml

.. code-block:: text

   CFD tools for examples/sample_cfd.toml
     prefix      /home/<user>/.local/share/micromamba/envs/ignisyeet-cfd
     SU2_CFD              ok       SU2 v8.3.0 "Harrier", The Open-Source CFD Code  [/home/<user>/.local/share/ignisyeet-su2-native/bin/SU2_CFD]
     mpirun               ok       mpirun (Open MPI) 5.0.11  [.../envs/ignisyeet-cfd/bin/mpirun]
     gmsh (python)        ok       gmsh 4.15.2  [.../envs/ignisyeet-cfd/bin/python]
     manifold3d (python)  ok       manifold3d installed  [.../envs/ignisyeet-cfd/bin/python]
   CFD mode is ready (12 cases for model Euler).

最終行が ``CFD mode is ready`` で終了ステータスが 0 なら準備ができている。ツールが足りないときは、不足を報告して終了ステータス 1 で終わる。
manifold3d の行は ``surface = "stl"`` のときだけ検査される。
``surface`` が ``"stl"`` か ``"step"`` で ``shape_symmetry`` が ``"auto"`` のときは、対称性が形状を調べるまで分からないので、ケース数は ``up to 20 cases`` のように全領域を解く場合の上限で示される。
``aero`` を CFD モードで実行したときも、計算を始める前に同じ検査が行われる。

``--mesh`` を付けると、実際にメッシュを作って品質を表示する。節点数、セル数、壁面の三角形数、セルの種類、品質（SICN の最小値と平均、0.05 未満のセル数）が出る。
``examples/sample_step.toml`` では、節点数 560092、セル数 2690207、壁面の三角形数 479830、SICN の最小値 0.071、平均 0.783、0.05 未満のセル 0 で、メッシュの生成に 101 秒かかった（20 スレッドのマシン）。
このメッシュは本番の計算にも使われる。

.. _sec-cfd-native:

SU2 の構築
~~~~~~~~~~

Linux では、``--cfd`` により SU2 8.3.0 がソースから、その場の CPU 向けに構築される。
構築を省いて移植できる conda-forge のバイナリを使うときは ``--su2-binary``\ （環境変数 ``IGNISYEET_SU2_BINARY=1``）を付ける。macOS では常にバイナリが使われる。
速さの違いは :ref:`sec-vperf-su2-native` に、インストーラのオプションと環境変数は :ref:`page-reference-installer` にある。どの SU2 が選ばれたかは ``cfd-check`` が表示する ``SU2_CFD`` のパスで分かる。
別の CPU に移した構築済みの SU2 は警告の対象になるので、その CPU 上で ``install.sh --cfd --force`` を実行して作り直すか、``--su2-binary`` に切り替える。

インストーラを使わずに、micromamba で同じ環境を作ることもできる。この場合の SU2 は conda-forge のバイナリである。

.. code-block:: console

   $ micromamba create -n ignisyeet-cfd -c conda-forge python=3.10 su2=8.3.0 gmsh=4.15.2 python-gmsh=4.15.2 openmpi=5
   $ ~/.local/share/micromamba/envs/ignisyeet-cfd/bin/python -m pip install manifold3d==3.5.4

.. _page-howto-cfd-table:

CFD で係数表を作る
------------------

.. _sec-cfd-run:

解くケースを決める
~~~~~~~~~~~~~~~~~~

設定の ``[aero.cfd]`` に、解く Mach 数 ``machs``、迎角 ``alphas_deg``、横滑り角 ``betas_deg`` を書く。この 3 つの組合せがケースの行列である。
``examples/sample_cfd.toml`` の例は次のとおりである。

.. code-block:: toml

   [aero]
   method = "cfd"

   [aero.cfd]
   prefix = "~/.local/share/micromamba/envs/ignisyeet-cfd"
   model = "euler"
   machs = [0.3, 0.8, 1.2, 2.0]    # 昇順
   alphas_deg = [0, 4, 8]          # 0 を含める
   wall_size = 0.002               # 機体表面の要素の大きさ [m]
   ranks_per_case = 4
   parallel_cases = 4

迎角は 0 を含める。横滑り角は 0 から始め、省略した ``betas_deg`` は ``[0]`` である。このときの係数表は軸対称表になる。0 以外の横滑り角を書くと、6 成分の一般表になる。
``alpha_mode`` は正負の迎角の扱いを決める。既定の ``"mirror"`` は、正の迎角を解いて奇関数部分と偶関数部分に分ける。

ケースの総数は、Mach 数の個数に Mach 数ごとのケース数を掛けたものである。
形状が対称でないと負の迎角と負の横滑り角も解くので、:math:`n_\alpha` 個の正の迎角と :math:`n_\beta` 個の正の横滑り角に対して、1 つの Mach 数で最大 :math:`(2n_\alpha+1)(2n_\beta+1)` ケースになる。
サンプル設定は 4 Mach 数に 3 つの迎角で、12 ケースである。

計算の前に、``cfd-check`` に ``--mesh`` を付けて資源を確かめる。メッシュの節点数とセル数、品質に続けて、並列度とメモリの見積もりが 1 行で出る。
次は ``examples/sample_step.toml`` を 30 GB のメモリのマシンで確かめたときの行である。

.. code-block:: text

   Resources: 4 ranks x 2 concurrent cases, about 9.0 GB per case (18.1 GB together), memory budget 27.0 GB (available memory, MemAvailable in /proc/meminfo)

同時に動かすケース数は、``[resources]`` の ``memory_gb``\ （既定は使えるメモリ）に収まるまで自動で減らされ、``note: aero.cfd: parallel_cases reduced from … to …`` が出る。
1 ケースの見積もりが予算を超えるときは、メッシュを作ったあと、計算の前にエラーで止まる。``wall_size`` を大きくするか、``ranks_per_case`` を減らす。
``ranks_per_case`` と ``parallel_cases`` の積は、使えるスレッド数以内にする。どちらも 0 にすると自動になる。資源の考え方は :ref:`sec-cfd-resources` にある。
サンプル機体の Euler では、1 ケースのメモリは 3.3 から 4.6 GB である。

係数表の作成と中断からの再開
~~~~~~~~~~~~~~~~~~~~~~~~~~~~

.. code-block:: console

   $ ignisyeet aero examples/sample_cfd.toml

ケースが順に解かれ、終わると ``out_cfd/`` に ``aero_table.csv`` と付随ファイル ``aero_table.json``、``cfd_cases.csv``、``cfd_report.json`` が書かれる。
失敗したケースと未収束のケースがあれば、最後にまとめて表示される。
続けて ``ignisyeet sim`` や ``ignisyeet dispersion`` を実行すると、この表が使われる。作った表を別の設定でも使うときは、``aero.method = "table"`` で読み込む。

:math:`M=0.3, 0.8, 1.2, 2.0` と :math:`\alpha=0, 4, 8^\circ` の 12 ケース（``sample_cfd.toml`` の行列）を 4 ランクで 1 ケースずつ解く時間は、合計で約 1.9 時間である。:math:`\alpha=0, 2, 4, 8^\circ` の 16 ケースでは全体で約 2.5 時間かかった（20 スレッドのデスクトップ CPU で、他の計算と同時に実行した測定。メッシュは ``wall_size`` 2 mm、142 万セル。``doc/data/cfd/run_summary.json``）。

途中で止めた（Ctrl-C、SIGTERM、時間切れ）あと、同じ設定で ``ignisyeet aero`` をもう一度実行すると、終わったケースは解き直さず、残りだけが解かれる。
最初からやり直すには、出力先の下の ``cfd`` ディレクトリ（``workdir`` で変えられる）を削除する。

解いたケースを確かめる
~~~~~~~~~~~~~~~~~~~~~~

まず ``cfd_cases.csv`` の ``status`` 列を見る。Euler で保存した例（``doc/data/cfd/sample_euler_cases.csv``。:math:`M=0.3`、:math:`\alpha=0, 2^\circ`）は次のようになる。

.. code-block:: text

   mach,alpha_deg,status,iterations,residual_first,residual_last,...,wall_seconds,warm_start,...
   0.3,0,converged,202,-4.81234362e0,-8.02204120e0,...,372.2,false,...
   0.3,2,converged,352,-3.31429450e-1,-8.96127494e0,...,723.7,true,...

``status`` の意味は次のとおりである。

.. list-table::
   :header-rows: 1
   :widths: 18 82

   * - 値
     - 意味と対処
   * - ``converged``
     - 残差が下がり、係数も落ち着いた。そのまま使える
   * - ``accepted``
     - Euler で、残差の目標には達しないが、係数が落ち着いているので受け入れた。使えるが、``cfd_report.json`` の警告を読む
   * - ``unconverged``
     - RANS で、係数の振動か残差の停滞のまま、直近の窓の平均を受け入れた。係数のばらつきを確かめてから使う
   * - ``failed``
     - SU2 のエラーや発散。表の作成では、失敗したケースを飛ばして補間する。原因は実行の最後の ``error[IY-F1nn]`` の行にある

受け入れの条件は :ref:`sec-cfd-int-convergence` と :ref:`sec-cfd-int-rans-accept` にある。
次に ``cfd_report.json`` で、失敗と未収束のケース（``failed``、``unconverged``、``filled``）、``warnings``、:math:`\alpha=\beta=0` の横方向の力 ``zero_angle``、
表が一定になる境界の Mach 数 ``mach_clamped_above`` と ``mach_clamped_below``、一般表で CFD の値を使う範囲 ``cfd_region``、ロール減衰の採用元 ``clp_source`` を見る。
``zero_angle`` は対称な機体では 0 のはずの値で、法線力に対する比が 5 % を超えると警告が出る。
項目の全体は後の節の「CFD の結果ファイル」にある。どこまでが CFD の値で、どこからがモデルで補った値かは :ref:`sec-cfd-trust` にある。

収束の履歴の例を :numref:`fig-cfd-convergence` に示す。
サンプル機体の Euler で、300 反復まで解くと、密度の残差が :math:`10^{-6}` に達するのは :math:`M=0.8` で約 100 反復、:math:`M=2` で約 80 から 100 反復である。
:math:`M=0.8` の揚力係数は振動しながら落ち着くので、残差が :math:`10^{-6}` に達した時点の法線力 :math:`C_N` は 300 反復の値より 0.6 % 低い。
残差の絶対値ではなく、係数の落ち着きを見るのはこのためである。
解の流れ場（Mach 数の分布と壁面の圧力係数）は :ref:`sec-cfd-flow` で見る。

.. _fig-cfd-convergence:

.. figure:: /_generated/plots/cfd_convergence.*
   :width: 100%

   サンプル機体の収束の履歴（Euler、``wall_size`` 2 mm、フィンの面 1 mm、142 万セル、一様流からの開始、300 反復）。左は密度の残差、右は :math:`\alpha=4^\circ` の揚力係数。残差が下がっても係数が落ち着くまで反復が続くことを見る。最初の 15 反復ほどの大きな振れは縦軸の範囲の外である。

計算で起きる主な失敗は、メモリ不足（SU2 がシグナルで止められる。``parallel_cases`` か ``ranks_per_case`` を減らす）、
ケースの発散や未収束（メッセージの理由を読む。平らな底面の Euler は収束しないことがある）、
最小または最大の Mach 数のケースがすべて失敗して表を補間できない場合（そのケースの失敗の理由を直す）である。
コードごとの説明は :ref:`page-reference-error-codes` にある。

RANS で解く
~~~~~~~~~~~

摩擦抗力も CFD から得るときは、``model = "rans"`` にする。``examples/sample_rans.toml`` を出発点にするほうが確実である。

.. code-block:: console

   $ ignisyeet cfd-check examples/sample_rans.toml --mesh
   $ ignisyeet aero examples/sample_rans.toml

``wall_size`` や ``iterations`` を設定しないと、RANS 用の既定値（壁面 5 mm、``yplus`` 1）が使われる。
使える形状と設定は限られる。``surface = "stl"``、``y=0`` について対称でないフィン配置の ``panel_mesh``、``roll_damping = true``、``yplus`` が 5 より大きく 30 より小さい場合は、計算を始める前にエラーで止まる。条件と理由は :ref:`sec-cfd-rans-use` にある。

1 ケースのメモリは Euler よりずっと大きい。サンプル機体の半モデルで ``wall_size`` 5 mm のとき、``cfd-check --mesh`` の見積もりは約 9.0 GB（73.5 万節点）で、``wall_size`` 4.8 mm（78.7 万節点）の実測のピークは 7804 MiB だった。
3 mm では見積もりが 22.3 GB になる（``doc/data/cfd/rans_mesh_sweep.csv``）。4 mm より細かくすると、メモリの目安を示す警告が出る。

``sample_rans.toml`` の 20 ケース（:math:`M=0.3, 0.8, 1.2, 2`、:math:`\alpha=0, \pm4, \pm8^\circ`）を 4 ランク、2 ケースずつ同時に解くと約 4.4 時間かかり、15 ケースが ``converged``、5 ケースが ``unconverged`` だった（``doc/data/cfd/rans_table_run.json``。測定の条件は同ファイルの ``machine`` 欄）。
``unconverged`` が出たら、実行の最後の警告 ``warning[IY-F508]`` と、``cfd_cases.csv`` の ``rans_verdict`` と ``residual_drop`` を読む。
係数表に使う前に、RANS の検証の範囲を確かめる（:ref:`page-vv-rans-verification`）。

.. _sec-cfd-roll-cases:

ロール減衰を CFD で求める
~~~~~~~~~~~~~~~~~~~~~~~~~

``[aero.cfd]`` に ``roll_damping = true`` を加えると、各 Mach 数で :math:`\alpha=\beta=0` のまま、ロール角速度 :math:`\hat p=\pm` ``roll_rate_hat``\ （既定 0.01）の 2 ケースが追加される。
機体が回る流れとして全領域で解くので、半領域のメッシュは使えず、Euler だけで使える。
2 ケースの差からロール減衰微係数 :math:`C_{lp}` が求まり、係数表に入る。
2 ケースの片方が失敗した Mach 数など、CFD の値を採用できない Mach 数は、部品積み上げ法の値に替わる。採用した側は ``cfd_report.json`` の ``clp_source`` で分かる。
指定しないときの :math:`C_{lp}` は部品積み上げ法の値である。

.. _page-howto-cad-step:

.. _sec-cfd-step:

CAD のモデル（STEP）を使う
--------------------------

CAD のモデル（STEP、IGES、BREP）の立体をそのまま CFD の壁面にする（``surface = "step"``）。
ランチラグ、レールボタン、カメラのポッド、ノズルのように STL の抽出では扱えない細部も、CAD にあれば壁面に入る。
曲面は三角形に近似せず、厳密な曲面のままメッシュにする。manifold3d は要らない。

飛翔計算の形状（基準面積、全長、フィンの寸法、部品積み上げ法の項）は、どの壁面のモードでも機体の入力（``rocket.stl`` または ``rocket.cad``）から抽出する。
CAD のモデルは、この入力と同じ機体でなければならない。``rocket.cad`` が機体の入力のときは、その CAD ファイルを ``step_file`` の既定にでき、照合の基準はその三角形分割になる。付属の ``examples/sample_step.toml`` は、サンプル機体にランチラグ 1 本とレールボタン 2 個を足した ``examples/sample_rocket.step``\ （mm 単位）を解く。

CAD から書き出す
~~~~~~~~~~~~~~~~

書き出しでは次の点を守る。

* **形式**： STEP（``.step``、``.stp``）、IGES（``.iges``、``.igs``）、BREP（``.brep``、``.brp``）。拡張子の大文字と小文字は区別しない。
* **単位**： STEP と IGES は、ファイルが宣言する長さの単位から m に換算して読む。BREP は単位を持たないので m として読む。
  宣言が誤っているときや BREP が m でないときは、``step_unit``\ （``"m"``、``"cm"``、``"mm"``、``"in"``、``"ft"``）で指定する。単位を宣言していない STEP と IGES では ``step_unit`` を使えないので、単位を宣言して書き出し直す。
  STL の単位（``rocket.stl_scale``）や ``rocket.cad_unit`` とは別に扱うので、機体の入力が mm で STEP が inch でもよい。
* **一つの閉じた立体**： 機体を一つのソリッドとして書き出す。複数のソリッドは一つに融合する。離れたまま残ったソリッドがあれば、すべてを壁面にして警告を出す。
  ソリッドのない面の集まりは縫い合わせて立体にするが、閉じた立体にならなければエラーになる。
* **座標系**： 機体の入力（``rocket.stl`` または ``rocket.cad``）と同じ座標系（同じ原点、同じノーズの向き、同じロールの向き）にする。``rocket.nose_direction`` は入力と CAD のモデルの両方に効く。
  CAD の中でモデルを動かしたり回したりしたあとで STL だけを書き出し直すと、後述の照合で止まる。
* **含めるもの**： 流れに触れる外形（ノーズ、胴体、フィン、ランチラグ、レールボタン、ポッド、底面から突き出たノズルなど）。モーターや内部の構造は含めない。
  機体の後端は、機体の入力の後端 :math:`x=L` にある平らな後ろ向きの面（底面）で閉じる。底面より後ろの面（突き出たノズル）は底面の扱いになり、力に入らない。
* **鋭い縁**： Euler では鋭くてもよい。RANS では、鋭い縁や狭いすき間（ランチラグの穴、レールボタンのつばと胴体の間）にプリズム層が沿えないことがあり、そのときは縁を少し丸めて書き出し直す。

設定に書く
~~~~~~~~~~

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

``step_file`` は設定を読むときに、ファイルがあることと拡張子を確かめる。``step_unit`` の ``"auto"``\ （既定）は、ファイルの宣言（BREP は m）に従う。
``step_file`` と ``step_unit`` を ``surface = "step"`` 以外で書くとエラーになる。CAD のファイルの内容を書き換えたときは、係数表を作り直す。

``base_treatment`` は Euler では必須である。``"fairing"`` は平らな底面の後ろに円すいを付け（形が変わる）、``"flat"`` は平らな底面を残す。RANS では使わない（:ref:`cfg-aero-cfd-base_treatment`）。
``fairing`` と ``flat`` の選び方は :ref:`page-theory-cfd-choose` にある。
``wall_size`` と ``fin_size`` は壁面とフィンの面の要素の大きさで、後述の照合の許容も ``wall_size`` で決まる。
``shape_symmetry``、``symmetry``、``z_mirror_mesh`` は、形状の検査に通った平面だけを使う。付属の ``sample_rocket.step`` は対称でなく、全領域を解く。

``aero`` などの設定の要約の ``cfd`` の行には、``surface step (sample_rocket.step)`` のようにファイル名が出る。

.. _sec-cfd-step-report:

読み込みと位置合わせを確かめる
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

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

各行の読み方は次のとおりである。``file`` は単位の宣言と換算の倍率で、倍率が想定と違えば ``step_unit`` を見直す。
``shape`` は融合の前後のソリッドと面の数、面積、体積で、融合後に 1 でなければ離れたソリッドが残っている。
``details`` は CAD だけにある細部の領域の数と面積、最も細い面の幅で、上の例の 3 領域はランチラグと 2 個のレールボタンである。
``domain`` は計算領域と対称性の判定で、上の例は鏡映対称でないので全領域を解く。``base`` は底面の処理である。
``registration`` は機体の入力と CAD の距離で、``max`` が ``allowed`` 以内なら通る。
``bounding box`` と ``offsets`` は包含箱の比較で、``offsets`` がすべて 0 に近ければ位置と向きがそろっている。

照合は、包含箱、ノーズの先端、入力の面と CAD の面の距離を比べる。:math:`L` を機体の入力から抽出した全長、:math:`h_w` を ``wall_size`` として、同じ機体とみなす条件は次のとおりである。

.. list-table::
   :header-rows: 1
   :widths: 24 76

   * - 条件
     - 内容
   * - 包含箱
     - 入力の包含箱が CAD のモデルの包含箱に :math:`0.01L` の余裕で収まる。CAD のモデルの包含箱は、どの軸でも入力の 1.25 倍を超えない
   * - ノーズ先端
     - :math:`x` の最小値の差が :math:`0.01L` 以下である
   * - 面の距離
     - 入力の面上の点が、すべて CAD のモデルの面から :math:`\max(2h_w,\,0.01L)` 以内にある。CAD の立体の内側に :math:`\max(h_w,\,0.002L)` より深く埋まった点（胴体に差し込んだ箱で表したフィンなど）は除き、除いた点は入力の点の 25 % 以下である
   * - CAD にだけある部分
     - 入力の面から :math:`\max(h_w,\,0.002L)` より離れた部分は、CAD の細部として許し、つながった領域の数と面積を報告する

許容は報告の ``allowed`` に表示される。許容より小さい形の違い（フィンの厚さの違いなど）は通る。
縫い合わせや形状の修復をしたときは ``note:`` の行に、離れたソリッドや ``"fairing"`` で形を変えたことは警告の行に出る。同じ内容は、係数表を作ったあとの ``cfd_report.json`` の ``step`` に入る。

照合に通らないときは、メッシュを作る前にエラーで止まり、不一致の理由と推定した原因が付く。原因は単位（``step_unit`` と ``rocket.stl_scale``、``rocket.cad_unit``）、向きか原点（機体の入力と同じ座標系で書き出し直す）、別の版のモデルのいずれかである（:ref:`IY-F071`）。

係数表を作る
~~~~~~~~~~~~

.. code-block:: console

   $ ignisyeet aero examples/sample_step.toml

CFD の実行、中断と再開、``cfd_cases.csv`` の読み方は STL の場合と同じである。CAD のモデルの係数表は、常に 6 成分の一般表になる。
付属の ``sample_rocket.step`` は鏡映対称でないので、全領域を解く。全領域は半領域の約 2 倍のセルで、負の迎角も解くので、``sample_cfd.toml``\ （:math:`\alpha=0, 4, 8^\circ` の 12 ケース、半領域）の約 1.9 時間よりかなり長くかかる。
``wall_size`` 6 mm（フィンの面 1 mm）では、``sample_rocket.step`` の全領域は 162 万セル、同じ大きさの ``panel_mesh`` の半モデルは 81 万セルだった（``doc/data/cfd/fin_resolution.csv``）。
細い面（ランチラグの管の壁など）には、その幅の 1/4（``wall_size`` の 1/8 を下限とする）まで細かい要素を置くので、細部の多いモデルではセルが増える。

RANS（``model = "rans"``）でも使える。プリズム層を CAD の壁面に積み、層の検査に通ったときだけメッシュとして使う。CAD の縁は鈍くしない。RANS は 1/4 領域を使わない。
プリズム層が鋭い縁に沿えないときは、``model = "euler"`` にするか、フィンの前縁と後縁、ランチラグやレールボタンの縁を少し丸める。狭いすき間で層の上面が交わったときは、すき間を広げるか埋める。
STEP の壁面の RANS の解は検証していないので、結果を使う前に :ref:`page-vv-rans-verification` を読む。

結果を使うときは、報告の ``registration`` が通っていること、``cfd_cases.csv`` の ``status`` に ``failed`` がないことを確かめる。
飛翔計算の部品積み上げ法の項は ``rocket.stl`` から求める。CAD の細部の圧力による抗力は CFD の軸力に入るので、``rocket.extra_cd`` で二重に数えない（付属の例は 0）。
Euler の摩擦抗力は STL の形状から求めるので、細部の摩擦抗力は入らない。

.. _page-reference-cfd-output-files:

CFD の出力ファイル
------------------

``aero.method = "cfd"`` は、係数表に加えて次のファイルを出力先に書く（``aero_drag.csv`` は書かない）。値の意味は :ref:`page-theory-cfd-choose` と :ref:`page-theory-cfd-coefficients` にある。

.. _sec-file-cfd:

``cfd_cases.csv``
~~~~~~~~~~~~~~~~~

解いたケースを 1 行に 1 つずつ書く。見出しは次のとおりである。

.. code-block:: text

   mach,alpha_deg,status,iterations,residual_first,residual_last,cn_raw,ca_cfd,nose_moment_raw,xcp_raw_m,cn,nose_moment,xcp_m,wall_seconds,warm_start,beta_deg,roll_hat,cx,cy,cz,cl,cm,cn_yaw,residual_reference,residual_drop,rans_verdict

.. list-table::
   :header-rows: 1
   :widths: 26 74
   :class: longtable

   * - 列
     - 内容
   * - ``mach`` ``alpha_deg`` ``beta_deg``
     - ケースの Mach 数、迎角 [deg]、横滑り角 [deg]
   * - ``status``
     - ``converged``、``accepted``、``unconverged``、``failed``\ （意味は前節）。``alpha_mode = "mirror"`` の軸対称表では、正負の解を合わせた行が ``combined`` で末尾に付く
   * - ``iterations``
     - 反復数（失敗したケースは 0。RANS は 2 次精度の段の反復数）
   * - ``residual_first`` / ``residual_last``
     - 履歴の最初の行と、判定した行の密度残差（対数）。判定した行は、収束で打ち切ったときはその時点の行、打ち切らなかったときは最後の行である。RANS では 2 次精度の段の履歴の値である
   * - ``cn_raw`` / ``ca_cfd``
     - SU2 の生の法線力係数 :math:`C_N` と軸力係数 :math:`C_A`\ （RANS の :math:`C_A` は壁面の ``CFx(wall)``、``rans_base = "cfd"`` では底面の ``CFx(base)`` を足した値。:ref:`sec-cfd-post`）
   * - ``nose_moment_raw`` / ``xcp_raw_m``
     - SU2 の生のノーズ先端まわりのモーメント（長さの次元 [m]。:math:`C_m` に相当する量）と、そこから求めた圧力中心 [m]
   * - ``cn`` / ``nose_moment`` / ``xcp_m``
     - 表が使う値。``offset`` では :math:`\alpha=0` の値を引いた値、``single`` では生の値と同じ、``mirror`` では生の行は空で ``combined`` の行にだけ入る。一般表のケースでは空
   * - ``wall_seconds``
     - 1 ケースの計算時間 [s]
   * - ``warm_start``
     - 別のケースの解から始めたかどうか（``true`` または ``false``）
   * - ``roll_hat``
     - ロール減衰のケースの無次元ロール角速度 :math:`\hat p`\ （それ以外は 0）
   * - ``cx`` ``cy`` ``cz`` ``cl`` ``cm`` ``cn_yaw``
     - 機体軸の 6 成分 :math:`C_X,C_Y,C_Z,C_l,C_m,C_n`\ （``cn_yaw`` が :math:`C_n`。法線力係数の ``cn`` と区別する）。結果のないケースは空
   * - ``residual_reference`` / ``residual_drop`` / ``rans_verdict``
     - RANS の受け入れの記録（:ref:`sec-cfd-int-rans-accept`）。``residual_reference`` は基準の最大値（密度の残差。対数）、``residual_drop`` は判定の根拠にした密度の残差の下がり [桁]、``rans_verdict`` は ``converged``、``limit_cycle``、``stalled`` のいずれか。Euler のケース、失敗したケース、``combined`` の行では空

``cfd_report.json``
~~~~~~~~~~~~~~~~~~~

CFD の実行の要約である。項目は次のとおりである。

.. list-table::
   :header-rows: 1
   :widths: 40 60
   :class: longtable

   * - 項目
     - 内容
   * - ``cases``
     - ケース数
   * - ``failed`` / ``unconverged``
     - 失敗したケース、未収束のケースの ``[Mach 数, 迎角]`` の一覧
   * - ``failed_cases`` / ``unconverged_cases``
     - それぞれのケースのディレクトリ
   * - ``stalled_cases``
     - RANS で残差が停滞したまま受け入れたケース。``case``\ （ディレクトリ名）と ``residual_drop``\ （密度の残差の下がり [桁]）の組の一覧
   * - ``filled``
     - 失敗して補間で埋めたケースの一覧
   * - ``zero_angle``
     - :math:`\alpha=\beta=0` の横方向の力
   * - ``wall_seconds`` / ``case_seconds``
     - 全体の計算時間 [s]、各ケースの計算時間の合計 [s]（ケースは同時に走るので、合計のほうが長い）
   * - ``mesh_nodes`` / ``mesh_cells``
     - メッシュの節点数とセル数
   * - ``alpha_mode``
     - ``mirror``、``offset``、``single``
   * - ``asymmetry``
     - 軸対称表で、測定した :math:`\alpha=0` の非対称性（Mach 数ごとの ``cn0`` と ``mom0``）
   * - ``ranks_per_case`` / ``parallel_cases`` / ``bytes_per_case``
     - 実際に使った 1 ケースあたりの MPI ランク数、同時に解いたケース数、1 ケースのメモリの見積もり [byte]
   * - ``est_wall_yplus``
     - RANS のとき、最初の層の高さから見積もった、機体の全長の位置での :math:`y^+`\ （:ref:`sec-cfd-prism`）
   * - ``table_kind``
     - 係数表の種類（``axisymmetric`` または ``general``）
   * - ``shape_symmetry`` / ``domain``
     - 実際に使った形状の対称性（``axisymmetric``、``pitch_plane``、``none``）と計算領域（``half`` または ``full``）
   * - ``betas_deg``
     - 解いた横滑り角
   * - ``cfd_region`` / ``model_outside_cfd_region``
     - 一般表で、CFD の値を使う範囲（``alpha_min_deg`` ``alpha_max_deg`` ``beta_min_deg`` ``beta_max_deg``）と、その外側がモデルの値かどうか（一般表では ``true``）
   * - ``mach_clamped_above`` / ``mach_clamped_below``
     - 表が端の値で一定になる境界の Mach 数。表の Mach 数の格子が解いた最大（最小）の Mach 数より上（下）まで広いときだけ入り、値は解いた最大（最小）の Mach 数である。CFD の表は Mach 数の方向に外挿しない
   * - ``clp`` / ``clp_source``
     - Mach 数ごとのロール減衰の CFD の値、部品積み上げ法の値、採用した側と理由（``clp``）と、全体の採用元（``cfd``、``barrowman``、``mixed``）
   * - ``diagnostics``
     - Mach 数ごとの零角度の傾き（``cna_pitch`` ``xcp_pitch`` ``cna_yaw`` ``xcp_yaw``）。一般表の入力ではなく診断用で、軸対称表ではヨーの値は入らない
   * - ``yaw_damping_from_pitch``
     - 横滑り角のデータのない一般表で、ヨー減衰の和がピッチと同じかどうか
   * - ``fin_cant_ref_deg``
     - CFD の形状に含まれるフィンのカント [deg]（ロールの強制項の基準）
   * - ``base_treatment``
     - 底面の扱い（``fairing``、``flat``）。壁面が STL か CAD のモデルのときと、Euler のときに入る。``surface = "step"`` の RANS では ``none``
   * - ``warnings``
     - 警告の一覧。各文字列の先頭にコード（``[IY-F5nn] …`` の形）が付く
   * - ``symmetry_check``
     - ``surface = "stl"`` と ``"step"`` の鏡映対称性の判定。``requested``\ （設定値）、``chosen``\ （採用した対称性）、平面ごとの検査 ``y`` と ``z``\ （``symmetric``、``max_mismatch`` [m]、``vertices``、``vertices_matched``、``triangulation_identical``、``tolerance``、``reason``）、メッシュの作り方 ``meshed``\ （``quarter``、``half``、``full``）、``reason``。
       ``step`` では、``vertices`` は鏡映した曲面上の点の数、``vertices_matched`` は曲面に一致した点の数、``tolerance`` は実際に使った許容（:ref:`sec-cfd-step-internals`）で、``triangulation_identical`` は常に ``false`` である
   * - ``fidelity``
     - ``surface = "stl"`` の形状の忠実度。``fidelity``\ （``stl_fidelity.json`` と同じ内容）、``union``\ （``union_report.json``）、``repair``\ （``repair_report.json``）を持つ
   * - ``step``
     - ``surface = "step"`` のときだけ入る。CAD のモデルの読み込み、位置合わせ、計算領域（:ref:`sec-cfd-step`）。上の報告の各行と同じ内容で、次を持つ。

       ``file``\ （CAD のファイル）、``format``\ （``step``、``iges``、``brep``）、``file_unit_m``\ （ファイルが宣言する単位 [m]、宣言がなければ ``null``）、``step_unit`` と ``unit_scale``\ （設定した単位とそこから決まる倍率）、
       ``solids_in_file`` と ``solids``\ （融合の前後のソリッドの数）、``faces`` と ``base_faces``\ （面の数と底面の面の数）、``volume`` [m\ :sup:`3`]、``area`` [m\ :sup:`2`]、``import_notes``\ （縫い合わせ、修復、融合の注記）、
       ``registration``\ （位置合わせ。下記）、``symmetry``\ （``symmetry_check`` と同じ）、``domain``\ （``full``、``half``\ （SU2 の半領域）、``half_mirrored``\ （半領域を鏡映した全領域）、``quarter_half``、``quarter_full``）、
       ``base_treatment``\ （``fairing``、``flat``、``none``）、``fairing``\ （円すいを付けた底面の位置 ``x``、半径 ``radius``、長さ ``length``、切った位置 ``cut_at``、底面の面積 ``base_face_area``）、
       ``smallest_feature`` と ``min_size``\ （最も小さい面の幅と最小の要素の大きさ [m]）、``warnings``。

       ``registration`` は、``threshold``\ （STL と CAD の面の距離の許容 [m]）、``detail_threshold``\ （CAD の細部とみなす距離 [m]）、
       ``stl_to_step`` と ``step_to_stl``\ （距離の統計 [m]。``max``、その位置 ``max_at``、``p99``、``rms``、標本数 ``samples``）、``stl_buried_fraction``\ （CAD の立体に埋まって除いた STL の点の割合）、
       ``step_only_regions`` と ``step_only_area``\ （CAD だけにある細部の領域の数と面積 [m\ :sup:`2`]）、``stl_bbox``、``step_bbox``、``bbox_offsets``\ （包含箱 ``[xmin, ymin, zmin, xmax, ymax, zmax]`` と CAD − STL の差 [m]）、
       ``problems``\ （合わない理由。通ったときは空）、``hint``\ （推定した原因）を持つ
   * - ``axial_breakdown``
     - RANS のとき、ロール減衰でないケースごとの軸力の内訳。``mach``、``alpha_deg``、``cfd_wall_ca`` と ``cfd_base_ca``\ （壁面と底面の ``CFx``。履歴に列がなければ ``null``）、
       ``barrowman_friction`` と ``barrowman_base``\ （その Mach 数の部品積み上げ法の摩擦と底面の抗力。比べるための値で、摩擦は表に足さない）、
       ``roughness_increment``\ （表に足す、設定の粗さによる摩擦の増分。:eq:`eq-cfd-rans-rough`）、``fin_edge_increment``\ （表に足す、設定のフィンの縁の増分。:eq:`eq-cfd-rans-fin`。``stl`` と ``step`` では 0）、``fin_te_base_increment``\ （表に足す、底面にそろったフィンの後縁の底面抗力。:eq:`eq-cfd-te-flush`。``stl`` と ``step`` で底面をモデルから取るときだけ。それ以外は 0）、``base_from_cfd``\ （表の底面の項が CFD の値か）を持つ。Euler では空

.. _sec-file-mesh:

作業ディレクトリ
~~~~~~~~~~~~~~~~

``output.dir`` の下の ``aero.cfd.workdir`` に、計算の作業ディレクトリが置かれる。次のファイルが入る。

.. list-table::
   :header-rows: 1
   :widths: 34 66

   * - パス
     - 内容
   * - ``m0.800_a04.00/`` のようなケースごとのディレクトリ
     - SU2 の設定、履歴、解、そのケースの結果 ``done.json``。ケースの条件（Mach 数、迎角、横滑り角）はディレクトリ名に入る
   * - ``mesh/mesh.su2``
     - 体積メッシュ
   * - ``mesh/mesh_stats.json``
     - メッシュの節点数、セル数、セルの種類ごとの数、品質（SICN）の統計、壁面の三角形数。RANS のメッシュはプリズム層の数、最初の層の高さ、厚さも持つ
   * - ``mesh/stl_fidelity.json`` ``mesh/union_report.json`` ``mesh/repair_report.json``
     - ``surface = "stl"`` のとき。形状の忠実度、重なった殻の和、STL の修復の報告。内容は ``cfd_report.json`` の ``fidelity`` に入る
   * - ``mesh/step_report.json``
     - ``surface = "step"`` のとき。CAD のモデルの読み込みと位置合わせの報告。内容は ``cfd_report.json`` の ``step`` に入る

同じ設定で実行し直すと、終わったケースは解き直さない。

.. _page-reference-aero-table-format:

係数表の形式
------------

``aero`` が書く ``aero_table.csv``、``aero_drag.csv``、``panel_cp.csv`` の列と項目を示す。
外部の係数表（``aero.method = "table"``）は、ここに書く形式で用意する。表の概念は :ref:`page-theory-aero-tables` と :ref:`page-theory-aero-coefficients` にある。

.. _sec-aero-table-format:

係数表（``aero_table.csv`` と ``aero_table.json``）
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

係数表は、CSV 本体と、拡張子を ``.json`` に替えた同名の付随ファイル（メタデータ）の 2 つで 1 組である。
``aero`` が ``out/aero_table.csv`` と ``out/aero_table.json`` を書き、``aero.method = "table"`` では同じ形式の外部の表を読み込む。
表の種類は付随ファイルの ``kind`` で決まり、``"axisymmetric"``\ （軸対称表。既定）と ``"general"``\ （一般表）がある。
軸対称表と一般表の関係は :ref:`sec-aero-table` と :ref:`page-theory-aero-coefficients` にある。

CSV の値は、``mach`` ``alpha_deg`` ``beta_deg`` の格子の値を除いて、指数表記で書かれる。桁数は、読み戻したときに元の倍精度の値とビットまで一致する最短の表記である。
1 行目が見出し、2 行目以降が格子点 1 つにつき 1 行で、空行は読み飛ばす。

.. _sec-file-axisym:

軸対称表の CSV
~~~~~~~~~~~~~~

見出しは次のとおりである。

.. code-block:: text

   mach,alpha_deg,cn,ca_on,ca_off,xcp,cna,damp_s0,damp_s1,damp_s2,cl_delta,clp,lag_l0,lag_l1,cl4,cnpa,cmpa

行は Mach 数が外側、迎角が内側の順（Mach 数ごとに迎角を小さい方から並べる）に、全格子点を 1 行ずつ並べる。
``alpha_deg`` は全迎角 :math:`\alpha_t` [deg] である（一般表では :math:`\alpha`。:ref:`sec-aero-alpha-beta`）。

.. list-table::
   :header-rows: 1
   :widths: 20 20 60

   * - 列
     - 単位
     - 内容
   * - ``mach``
     - 無次元
     - Mach 数
   * - ``alpha_deg``
     - deg
     - 全迎角 :math:`\alpha_t`
   * - ``cn``
     - 無次元
     - 法線力係数 :math:`C_N`
   * - ``ca_on`` / ``ca_off``
     - 無次元
     - 軸力係数 :math:`C_A`\ （燃焼中 / 燃焼後）
   * - ``xcp``
     - m
     - 圧力中心 :math:`x_{cp}`\ （ノーズ先端から後方へ測る）
   * - ``cna``
     - 1/rad
     - 法線力係数の傾き :math:`C_{N\alpha}`
   * - ``damp_s0`` / ``damp_s1`` / ``damp_s2``
     - 順に m\ :sup:`0`、m\ :sup:`1`、m\ :sup:`2` を含む量
     - ピッチ減衰の和 :math:`S_0, S_1, S_2`\ （:eq:`eq-moment-damp`）
   * - ``cl_delta``
     - 1/rad
     - フィンのカント 1 rad あたりのロールモーメント係数 :math:`C_{l\delta}`
   * - ``clp``
     - 無次元
     - ロール減衰微係数 :math:`C_{lp}=\partial C_l/\partial\hat p`\ （:math:`\hat p=pd/(2V)`。負）
   * - ``lag_l0``
     - m
     - 遅れの和 :math:`L_0=(2/S)\int A\,\mathrm{d}x`\ （胴体とフィンの見かけの質量。:eq:`eq-lag-sums`）
   * - ``lag_l1``
     - m\ :sup:`2`
     - 遅れの和 :math:`L_1=(2/S)\int A\,x\,\mathrm{d}x`\ （ノーズ先端まわり。:math:`L_1/L_0` が見かけの質量の中心）
   * - ``cl4``
     - 無次元
     - 誘導ロールの係数 :math:`C_{l4}`\ （:math:`\Delta C_l=C_{l4}\alpha_t^2\sin4(\phi_c-\phi_f)`、:math:`\alpha_t` は rad。:eq:`eq-induced-roll`）
   * - ``cnpa``
     - 1/rad
     - Magnus 力の傾き :math:`C_{Np\alpha}^{*}`\ （:math:`C_{N,\mathrm{Magnus}}=C_{Np\alpha}^{*}\hat p\sin\alpha_t`。McCoy の :math:`pd/V` で無次元化した値の 2 倍。:eq:`eq-magnus`）
   * - ``cmpa``
     - 1/rad
     - 静的な圧力中心まわりの Magnus のモーメントの傾き :math:`C_{mp\alpha}^{*}`\ （:eq:`eq-magnus-moment`。0 なら Magnus 力が圧力中心に働くだけ）

既定の格子は、Mach 数が :math:`M=0,0.02,\dots,3.0`\ （151 点）、全迎角が :math:`\alpha_t=0^\circ,1^\circ,\dots,90^\circ`\ （91 点）である
（設定 ``aero.mach_min`` ``mach_max`` ``mach_step`` と ``alpha_max_deg`` ``alpha_step_deg``。:ref:`[aero] の設定項目 <sec-config-aero>`）。
飛翔計算は 0 から 90° の :math:`\alpha_t'` で表を引くので、既定の表では範囲を超えない。``alpha_max_deg`` を小さくした表で範囲を超えた迎角は外挿し、その結果は使えないもの（終了ステータス 3）とする（:ref:`sec-aero-interp`）。

``lag_l0`` と ``lag_l1`` は、部品積み上げ法では機体の形状から求め（:ref:`sec-damping`）、パネル法と CFD の表にも同じ形状の値を入れる。
``cl4``、``cnpa``、``cmpa`` は、部品積み上げ法、パネル法、CFD のどれも推算せず 0 を書く。値を使うには外部の表に書く。5 列の値の飛翔計算での使い方は :ref:`sec-aero-unsteady` にある。

.. _sec-file-general:

一般表の CSV
~~~~~~~~~~~~

見出しは次のとおりである。

.. code-block:: text

   mach,alpha_deg,beta_deg,cx_on,cx_off,cy,cz,cl,cm,cn,cl_delta,clp,sq0,sq1,sq2,sr0,sr1,sr2,lag_l0,lag_l1,cnpa,cmpa

行は Mach 数が最も外側、迎角が次、横滑り角が最も内側の順に、全格子点を 1 行ずつ並べる。
``alpha_deg`` は迎角 :math:`\alpha`、``beta_deg`` は横滑り角 :math:`\beta` である（定義は :ref:`sec-aero-angles`）。
行は 3 つの格子値に続く 19 個の値を持つ。

.. list-table::
   :header-rows: 1
   :widths: 20 20 60

   * - 列
     - 単位
     - 内容
   * - ``mach``, ``alpha_deg``, ``beta_deg``
     - 無次元、deg、deg
     - 格子点の Mach 数、迎角 :math:`\alpha`、横滑り角 :math:`\beta`
   * - ``cx_on`` / ``cx_off``
     - 無次元
     - 機体座標の :math:`x_b` 方向の力係数 :math:`C_X`\ （燃焼中 / 燃焼後）
   * - ``cy`` / ``cz``
     - 無次元
     - :math:`y_b` 方向、:math:`z_b` 方向の力係数 :math:`C_Y`, :math:`C_Z`
   * - ``cl`` / ``cm`` / ``cn``
     - 無次元
     - ノーズ先端まわりのモーメント係数 :math:`C_l`, :math:`C_m`, :math:`C_n`\ （:math:`qSd` で無次元化。``cn`` は 6 成分のヨーモーメントで、軸対称表の法線力係数 :math:`C_N` ではない。:ref:`sec-aero-cn-cz`）
   * - ``cl_delta``
     - 1/rad
     - カント 1 rad あたりのロールモーメント係数 :math:`C_{l\delta}`\ （表のカントの基準 ``fin_cant_ref_deg`` との差の分だけ飛翔計算が足す）
   * - ``clp``
     - 無次元
     - ロール減衰微係数 :math:`C_{lp}`
   * - ``sq0`` / ``sq1`` / ``sq2``
     - ``damp_s0`` から ``damp_s2`` と同じ
     - ピッチの減衰の和 :math:`S_0^q,S_1^q,S_2^q`
   * - ``sr0`` / ``sr1`` / ``sr2``
     - ``damp_s0`` から ``damp_s2`` と同じ
     - ヨーの減衰の和 :math:`S_0^r,S_1^r,S_2^r`
   * - ``lag_l0`` / ``lag_l1`` / ``cnpa`` / ``cmpa``
     - 軸対称表と同じ
     - 遅れの和と Magnus 力とモーメントの傾き（軸対称表の同名の列と同じ意味。``cmpa`` は等価な圧力中心まわり）

一般表では、単一の圧力中心 :math:`x_{cp}` や :math:`C_{N\alpha}` を飛翔計算に使わない。
6 成分の定義、符号、基準点は :ref:`page-theory-aero-coefficients` と :ref:`sec-aero-forces`、減衰の和の使い方は :eq:`eq-moment-damp` にある。
一般表は ``cl4`` の列を持たない。誘導ロールは :math:`C_l` の列に含める（軸対称表から展開した表では、展開のときに加える）。

迎角 :math:`-180^\circ\le\alpha\le180^\circ` と横滑り角 :math:`-90^\circ\le\beta\le90^\circ` の格子は不等間隔である。
:math:`\pm30^\circ` の内側は 2° 刻み、外側は 10° 刻みで、迎角は 61 点、横滑り角は 43 点になる。
軸対称表から展開した一般表（部品積み上げ法、パネル法、外部の軸対称表）は、この格子を使う。CFD の一般表は同じ格子に値を埋めて作る（:ref:`sec-cfd-general`）。
外部の一般表の格子は不等間隔でもよく、付随ファイルの ``alphas_deg`` と ``betas_deg`` が決める。

Mach 数の刻みは、作り方で異なる。部品積み上げ法は ``aero.general_mach_step``\ （既定 0.05。``method = "barrowman"`` かつ ``table_kind = "general"`` のときだけ効く）、
パネル法と外部の軸対称表はその軸対称表の Mach 数の格子、CFD は ``aero.cfd.general_mach_step``\ （既定 0.1。0 のとき ``aero.mach_step``）である。
サンプルの機体の一般表（部品積み上げ法）は、61（Mach 数）× 61（迎角）× 43（横滑り角）= 160003 行になる。
軸対称表から展開した一般表は、軸対称表より情報を増やさない（飛翔の比較は :ref:`sec-vaero-table` に示す）。

.. _sec-file-sidecar:

付随ファイル ``aero_table.json``
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

付随ファイルは、表の基準量、格子、種類、入力の識別子を持つ JSON である。書き出すときは、次の項目をこの順に書く。

.. list-table::
   :header-rows: 1
   :widths: 24 14 62
   :class: longtable

   * - 項目
     - 読み込み
     - 内容
   * - ``format_version``
     - 任意
     - 形式のバージョン。書き出すときは常に 4。読み込み時はこの値を使わず、CSV の列数で形式を判定する
   * - ``kind``
     - 任意
     - ``"axisymmetric"``\ （既定）または ``"general"``
   * - ``source_hash``
     - 必須
     - 入力を識別する文字列。``aero`` が書く表では、同じ機体と空力の設定で作った表かを区別するための値になる。外部の表では任意の文字列でよい
   * - ``ref_area``
     - 必須
     - 基準面積 [m\ :sup:`2`]
   * - ``ref_diameter``
     - 必須
     - 基準直径 :math:`d` [m]
   * - ``length``
     - 必須
     - 全長 [m]。``motor.aft_x`` を省略したときのモータ後端位置はこの値になる
   * - ``machs``
     - 必須
     - Mach 数の格子（CSV の ``mach`` 列の値を昇順に）
   * - ``alphas_deg``
     - 必須
     - 迎角の格子 [deg]。軸対称表では全迎角 :math:`\alpha_t`\ （0 以上）、一般表では :math:`\alpha`
   * - ``betas_deg``
     - 任意
     - 一般表の横滑り角の格子 [deg]（:math:`-90^\circ` から :math:`90^\circ`）。軸対称表では空
   * - ``fin_cant_ref_deg``
     - 任意
     - 表の :math:`C_l` に含まれるフィンのカント :math:`\delta_{\mathrm{ref}}` [deg]。既定は 0（:ref:`sec-aero-table`）
   * - ``moment_ref``
     - 任意
     - モーメントの基準点。現在は ``"nose_tip"``\ （既定）だけである（:ref:`sec-aero-moment-ref`）
   * - ``fin_roll_deg``
     - 任意
     - 軸対称表の誘導ロール ``cl4`` の基準になる 1 枚のフィンのロール角 :math:`\phi_f` [deg]（:math:`+y_b` から :math:`+z_b` へ測る。:eq:`eq-induced-roll`）。既定は 0。有限でなければエラー
   * - ``fin_count``
     - 任意
     - フィンの枚数。``cl4`` は 4 枚（十字形）のときと、この項目がないときだけ使う。部品積み上げ法、パネル法、CFD の表と、形状から遅れの和を補った外部の表には、形状のフィンの枚数を書く
   * - ``extrapolation``
     - 必須
     - 迎角と横滑り角が表の範囲外のときの扱い。``"linear"``\ （線形外挿。符号と範囲を制限する）または ``"clamp"``\ （端の値で一定）。Mach 数は、この値にかかわらず端の値で一定（:ref:`sec-aero-interp`）

部品積み上げ法、パネル法、CFD の軸対称表の ``fin_cant_ref_deg`` は、設定したカントである。CFD の一般表では、``stl`` と ``step`` のとき設定したカント、``panel_mesh`` のとき 0 である（形状にカントがないため）。この値は ``cfd_report.json`` の ``fin_cant_ref_deg`` にも出る。

読み込み時の検査
~~~~~~~~~~~~~~~~

表を読み込むときは、次を検査する。違反があるとエラーにして、メッセージにファイル名（行に関するものは行番号も）を示す。

* 付随ファイルと CSV が読めること。
* 基準面積 ``ref_area``、基準直径 ``ref_diameter``、全長 ``length`` が有限で正であること。``fin_cant_ref_deg`` と ``fin_roll_deg`` が有限であること。
* ``moment_ref`` が ``"nose_tip"`` であること。ほかの値は、重心まわりへの移し替えをノーズ先端まわりのモーメントから行うので受け付けない。
* ``machs`` と ``alphas_deg``\ （一般表では ``betas_deg`` も）が空でなく、有限で、厳密に増加すること。
* 迎角の格子が 0 を含むこと。軸対称表の ``alphas_deg`` は 0 から始まり、一般表の ``alphas_deg`` と ``betas_deg`` は、最初の値と最後の値の間に 0 を含む。
  零揚力抗力、法線力傾斜、迎角 0 の圧力中心と静安定余裕（:ref:`sec-static-margin-general`）を、外挿ではなく表の値から求めるためである。
* CSV の行数が、``machs`` と ``alphas_deg``\ （一般表では ``betas_deg`` も）の積に一致すること。
* 各行の格子の座標が、付随ファイルの格子から決まる位置の値に一致すること。並びが違う行は、行番号と期待される格子点を示してエラーにする（比較の許容は相対 :math:`10^{-6}`）。
* 格子の座標の列を含むすべてのセルが有限であること。``NaN`` と ``inf`` は、行と列の位置を示してエラーにする。
* 値が数として読めない行、列数が合わない行がないこと。

CSV の見出しは、列の名前と順序が上の見出しと一致していなければならない（大文字小文字と前後の空白は無視する）。違うとエラーになる（:ref:`IY-A044`）。

.. _sec-file-v1:

列の少ない形式の表
~~~~~~~~~~~~~~~~~~

各行の値の数は見出しの列数に一致しなければならない。軸対称表は 17 列（格子の 2 列に値 15 個）、一般表は 22 列（格子の 3 列に値 19 個）の形式か、:numref:`tbl-aero-table-short` の列の組の形式である。足りない列は 0 とする。

.. _tbl-aero-table-short:

.. list-table:: 列の少ない形式の表
   :header-rows: 1
   :widths: 22 18 60

   * - 表
     - 列数
     - 足りない列
   * - 軸対称表（``format_version`` 1）
     - 10（値 8 個）
     - ``cl_delta`` ``clp`` ``lag_l0`` ``lag_l1`` ``cl4`` ``cnpa`` ``cmpa``。付随ファイルに ``format_version`` がなくてもよい
   * - 軸対称表（``format_version`` 2）
     - 12（値 10 個）
     - ``lag_l0`` ``lag_l1`` ``cl4`` ``cnpa`` ``cmpa``
   * - 軸対称表（``format_version`` 3）
     - 16（値 14 個）
     - ``cmpa``
   * - 一般表（15 個の値）
     - 18
     - ``lag_l0`` ``lag_l1`` ``cnpa`` ``cmpa``
   * - 一般表（18 個の値）
     - 21
     - ``cmpa``

見出しは、``format_version`` 4 の見出しの先頭から、その列数までと一致しなければならない。

``aero.method = "table"`` で読み込んだ表のロール項がすべて 0 のとき（``format_version`` 1 の表を含む）は、``aero``、``sim``、``dispersion`` のどれでも、現在の STL のフィンから部品積み上げ法のロール項（:math:`C_{l\delta}` と :math:`C_{lp}`）を求めて表に補い、端末に注意書きを表示する。
形状を抽出できないときは、注意書きを表示してロール項を 0 のままにし、機体はロールしない。フィンがなければ補う値も 0 で、注意書きは出ない。
ロール項を含む表は、そのまま使う。補った値は読み込んだ表のメモリ上だけに入り、元のファイルは書き換えない。

遅れと Magnus の列を持たない外部の表（:numref:`tbl-aero-table-short` の ``format_version`` 1、2 と 15 個の値の一般表）には、ロール項と同じく、現在の STL の形状から部品積み上げ法の遅れの和（胴体とフィンの見かけの質量。:ref:`sec-damping`）を求めて補い、警告を出す（:ref:`IY-O525`）。
形状を抽出できないときは遅れの和を 0 のまま飛ばし、その旨の警告を出す（:ref:`IY-O527`）。
どちらの場合も Magnus 力とモーメント、誘導ロールは 0 で、回転による法線力（減衰の和から求める。:eq:`eq-cnq-force`）は働く。

**実測や CFD のピッチ減衰を使う表**：強制振動の風洞試験や CFD、実射の同定で得たピッチ減衰 :math:`C_{mq}+C_{m\dot\alpha}` は、局所的な迎角の項だけでなく、見かけの質量の遅れと対流の回転の項（:eq:`eq-lag-dyn-section`）をすでに含む。
この値を使う表では ``lag_l0`` と ``lag_l1`` を 0 にし、減衰を減衰の和に入れる。
0 は列を持つ形式（``format_version`` 3 以降）に明示して書く。明示した 0 の列は補わないが、列のない形式では形状から遅れの和が補われ、見かけの質量の項を二重に数える（そのときの警告はこの点を示す）。
基準の重心 :math:`x_{\mathrm{ref}}` で測った :math:`C_{mq}+C_{m\dot\alpha}`\ （:math:`qd/(2V)` あたり）から、重心まわりの和 :math:`K=-(C_{mq}+C_{m\dot\alpha})\,d^2/4` を作り、
法線力の傾きと圧力中心を表す :math:`S_0,S_1` はそのままに :math:`S_2=K+2x_{\mathrm{ref}}S_1-x_{\mathrm{ref}}^2S_0` とする（:eq:`eq-damping`）。こうすると、重心が動いたときの減衰の変化は局所的な迎角の和の形で近似される。
:math:`C_{mq}` と :math:`C_{m\dot\alpha}` が別々に分かっているときは、:math:`C_{m\dot\alpha}` を遅れの和で表してもよい。
:math:`\dot u_z=V\dot\alpha` で遅れのモーメントは :math:`C_{m\dot\alpha}=2(x_{\mathrm{ref}}L_0-L_1)/d^2` にあたるので、:math:`L_0=0`、:math:`L_1=-C_{m\dot\alpha}d^2/2` とする（重心によらない純粋なモーメントになり、対流の回転の力も 0 になる）。
遅れの和は対流の回転の項にも同じ値で効き、:math:`C_{mq}` に :math:`C_{m\dot\alpha}` と同じ値を足すので、減衰の和の側は :math:`K=-(C_{mq}-C_{m\dot\alpha})\,d^2/4` から :math:`S_2` を作る。

.. _sec-aero-table-format-external:

外部の表を作るときの要点
~~~~~~~~~~~~~~~~~~~~~~~~

外部の表（``aero.method = "table"``）は、上の形式のとおりに CSV と付随ファイルを用意する。使い方は :ref:`sec-usage-external-table` にある。確認する点は次のとおりである。

* 付随ファイルは CSV と同じ名前で拡張子が ``.json`` のファイルで、必須の項目（``source_hash`` ``ref_area`` ``ref_diameter`` ``length`` ``machs`` ``alphas_deg`` ``extrapolation``）をすべて書く。
* 軸対称表の ``alpha_deg`` は全迎角 :math:`\alpha_t`、一般表の ``alpha_deg`` は迎角 :math:`\alpha`、``beta_deg`` は横滑り角 :math:`\beta` である。軸対称表の ``cn`` は法線力係数、一般表の ``cn`` はヨーモーメント係数 :math:`C_n` で、意味が違う（:ref:`sec-aero-cn-cz`）。
* モーメントはノーズ先端まわり（``moment_ref = "nose_tip"``）で、:math:`qSd` で無次元化する。
* 一般表は ``kind = "general"`` とし、``betas_deg`` を書く。格子は不等間隔でもよいが、各軸は厳密に増加させ、CSV の行は Mach 数、迎角、横滑り角（最も内側）の順に並べる。軸対称表も Mach 数、全迎角の順に並べる。
* 軸対称表の ``alphas_deg`` は 0 から始め、一般表の ``alphas_deg`` と ``betas_deg`` は 0 を含める。格子の座標を含むすべてのセルは有限にする。
* 格子の外の引き出しは、迎角・横滑り角の最初の格子点より小さい側も、最後の格子点より大きい側も、範囲外として数える（``aero_table_range`` の ``angle_beyond``。:ref:`sec-file-summary`）。格子点が 1 つだけの軸では、その値以外はすべて範囲外である。
* 軸対称な外部の表は、``aero.table_kind = "general"`` で一般表に展開できる（展開した表は書き出さない）。外部の表が最初から一般表なら、そのまま使う。
* 軸対称表の誘導ロール ``cl4`` は、付随ファイルの ``fin_roll_deg`` のフィンの面を基準にした値で、フィンが 4 枚の機体（``fin_count``）にだけ使う。
  ``cl4`` を書くときは ``fin_count`` も書く。ないときは 4 枚とみなして警告を出す（:ref:`IY-O526`）。
  Magnus 力の ``cnpa`` は :math:`\hat p=pd/(2V)` についての傾き（McCoy の :math:`C_{Np\alpha}` の 2 倍）で、正の向きは :math:`\bm{u}\times\bm{e}_x` である（:eq:`eq-magnus`）。``cmpa`` は静的な圧力中心まわりの Magnus のモーメント（McCoy の :math:`C_{Mp\alpha}` の向き。:eq:`eq-magnus-moment`）である。

.. _sec-file-drag:

``aero_drag.csv``
~~~~~~~~~~~~~~~~~

Mach 数ごとの抗力の内訳である。``method = "barrowman"`` と ``"panel"`` の ``aero`` が書く（``"cfd"`` は書かない）。
部品積み上げ法の見出しは次のとおりである。

.. code-block:: text

   mach,reynolds,friction,nose,base,fins,boattail,extra,cd_off,cd_on,cna,xcp

``reynolds`` は Reynolds 数、``friction`` ``nose`` ``base`` ``fins`` ``boattail`` ``extra`` は燃焼後の抗力係数の内訳（摩擦、ノーズ、底面、フィン、ボートテール、``extra_cd``）、
``cd_off`` と ``cd_on`` は燃焼後と燃焼中の合計である。``cna``\ （1/rad）と ``xcp``\ （m、ノーズ先端から）は :math:`\alpha=0` での値である。

``method = "panel"`` では内訳を持たず、見出しは ``mach,cd_off,cd_on,cna,xcp`` である。``cd_off`` と ``cd_on`` は、表の :math:`\alpha=0` の軸力係数 ``ca_off`` と ``ca_on`` である。

.. _sec-file-panel-cp:

``panel_cp.csv``
~~~~~~~~~~~~~~~~

``method = "panel"`` が書く、表面パネルの圧力係数である。見出しは ``x,y,z,nx,ny,nz,area,cp,part`` である。
``x`` ``y`` ``z`` はパネルの位置 [m]、``nx`` ``ny`` ``nz`` は法線、``area`` は面積 [m\ :sup:`2`]、``cp`` は圧力係数、``part`` は部位（``body`` または ``fin``）である。
値は迎角 4°、最も低い亜音速 Mach 数（既定は :math:`M=0`）の解である。``python/plot.py panel`` で作図する（:ref:`page-theory-panel`）。
