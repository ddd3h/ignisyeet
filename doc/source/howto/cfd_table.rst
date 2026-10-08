.. _page-howto-cfd-table:

CFD で係数表を作る
====================

SU2 で機体まわりの流れを解き、その結果から空力係数表を作る手順を示す。
このページを終えると、``aero.method = "cfd"`` で係数表を作り、解いたケースの状態を確かめ、中断した計算を再開できる。
CAD のモデルから作るときは :ref:`page-howto-cad-step` を、CFD にするかどうかの判断は :ref:`page-theory-aero-choose` を参照する。

前提
------

* CFD のツールが入っていて、``ignisyeet cfd-check`` が ``CFD mode is ready`` を返す（:ref:`page-howto-cfd-setup`）。
* STL と機体の設定が済んでいる（:ref:`page-howto-own-rocket`）。サンプル機体では ``examples/sample_cfd.toml`` をそのまま使える。
* 計算に使えるメモリと時間の見当がある。サンプル機体の Euler で、1 ケースのメモリは 3.3 から 4.6 GB である。
  :math:`M=0.3, 0.8, 1.2, 2.0` と :math:`\alpha=0, 4, 8^\circ` の 12 ケースを 4 ランクで 1 ケースずつ解くと約 1.9 時間かかった（20 スレッドのデスクトップ CPU で、他の計算と同時に実行した測定。メッシュは ``wall_size`` 2 mm、142 万セル。``doc/data/cfd/run_summary.json``）。

手順
------

.. _sec-cfd-run:

1. 解くケースの行列を決める
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

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

* 迎角は 0 を含める。横滑り角は 0 から始める。省略した ``betas_deg`` は ``[0]`` で、このときの係数表は軸対称表である。
  0 以外の横滑り角を書くと、6 成分の一般表になる。
* ``alpha_mode`` は、正負の迎角の扱いを決める。既定の ``"mirror"`` は、正の迎角を解いて奇関数部分と偶関数部分に分ける。
  選び方は :ref:`page-theory-cfd-choose` にある。
* ケースの総数は、Mach 数の個数に Mach 数ごとのケース数を掛けたものである。形状が対称でないと、負の迎角と負の横滑り角も解くので、:math:`n_\alpha` 個の正の迎角と :math:`n_\beta` 個の正の横滑り角に対して、1 つの Mach 数で最大 :math:`(2n_\alpha+1)(2n_\beta+1)` ケースになる。
  サンプル設定は上の 4 Mach 数に 3 つの迎角で、12 ケースである。
* ロール減衰を CFD で求める場合は、``roll_damping = true`` を加える（手順 7）。

2. 資源と準備を確かめる
~~~~~~~~~~~~~~~~~~~~~~~~

.. code-block:: console

   $ ignisyeet cfd-check examples/sample_cfd.toml --mesh

期待する結果： メッシュの節点数とセル数、品質（SICN の最小値と平均）に続けて、並列度とメモリの見積もりが 1 行で出る。
次は ``examples/sample_step.toml`` を 30 GB のメモリのマシンで確かめたときの行である。

.. code-block:: text

   Resources: 4 ranks x 2 concurrent cases, about 9.0 GB per case (18.1 GB together), memory budget 27.0 GB (available memory, MemAvailable in /proc/meminfo)

* 同時に動かすケース数は、``[resources]`` の ``memory_gb``\ （既定は使えるメモリ）に収まるまで自動で減らされ、``note: aero.cfd: parallel_cases reduced from … to …`` が出る。
* 1 ケースの見積もりが予算を超えるときは、メッシュを作ったあと、計算の前にエラーで止まる（:ref:`IY-F002`）。``wall_size`` を大きくするか、``ranks_per_case`` を減らす。
* ``ranks_per_case`` と ``parallel_cases`` の積は、使えるスレッド数以内にする。どちらも 0 にすると自動になる。
  資源の考え方は :ref:`sec-cfd-resources` にある。

3. 係数表を作る
~~~~~~~~~~~~~~~~

.. code-block:: console

   $ ignisyeet aero examples/sample_cfd.toml

期待する結果： ケースが順に解かれ、終わると ``out_cfd/`` に ``aero_table.csv`` と付随ファイル ``aero_table.json``、``cfd_cases.csv``、``cfd_report.json`` が書かれる。
失敗したケースと未収束のケースがあれば、最後にまとめて表示される。
続けて ``ignisyeet sim`` や ``ignisyeet dispersion`` を実行すると、この表が使われる。
作った表を別の設定でも使うときは、``aero.method = "table"`` で読み込む（:ref:`page-howto-external-table`）。

4. 解いたケースを確かめる
~~~~~~~~~~~~~~~~~~~~~~~~~~

``cfd_cases.csv`` の ``status`` 列を見る。Euler で保存した例（``doc/data/cfd/sample_euler_cases.csv``。:math:`M=0.3`、:math:`\alpha=0, 2^\circ`）は次のようになる。

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
     - RANS で、係数の振動か残差の停滞のまま、直近の窓の平均を受け入れた。``cfd_report.json`` の ``unconverged`` と ``stalled_cases`` に載る。係数のばらつきを確かめてから使う
   * - ``failed``
     - SU2 のエラーや発散。表の作成では、失敗したケースを飛ばして補間する。原因は実行の最後の ``error[IY-F1nn]`` の行にある

受け入れの条件の考え方は :ref:`sec-cfd-int-convergence` と :ref:`sec-cfd-int-rans-accept` にある。
``cfd_report.json`` では、まず次の項目を見る。

.. list-table::
   :header-rows: 1
   :widths: 30 70

   * - 項目
     - 見るもの
   * - ``failed`` ``failed_cases`` ``unconverged`` ``filled`` ``warnings``
     - 失敗、未収束、補間で埋めたケース、警告
   * - ``zero_angle``
     - :math:`\alpha=\beta=0` の横方向の力。対称な機体では 0 のはずの値で、法線力に対する比が 5 % を超えると警告が出る
   * - ``mach_clamped_above`` ``mach_clamped_below``
     - 解いた Mach 数の範囲より表の格子が広いとき、表が一定になる境界の Mach 数
   * - ``cfd_region`` ``model_outside_cfd_region``
     - 一般表で、どこまでが CFD の値か
   * - ``clp`` ``clp_source``
     - ロール減衰を CFD の値にしたか、部品積み上げ法の値にしたか

項目の全体は :ref:`page-reference-cfd-output-files` にある。どこまでが CFD の値で、どこからがモデルで補った値かは :ref:`sec-cfd-trust` にある。

収束の履歴の例を :numref:`fig-cfd-convergence` に示す。
サンプル機体の Euler で、300 反復まで解くと、密度の残差が :math:`10^{-6}` に達するのは :math:`M=0.8` で約 100 反復、:math:`M=2` で約 80 から 100 反復である。
:math:`M=0.8` の揚力係数は振動しながら落ち着くので、残差が :math:`10^{-6}` に達した時点の法線力 :math:`C_N` は 300 反復の値より 0.6 % 低い。
残差の絶対値ではなく、係数の落ち着きを見るのはこのためである。
解の流れ場（Mach 数の分布と壁面の圧力係数）は :ref:`sec-cfd-flow` で見る。

.. _fig-cfd-convergence:

.. figure:: /_generated/plots/cfd_convergence.*
   :width: 100%

   サンプル機体の収束の履歴（Euler、``wall_size`` 2 mm、フィンの面 1 mm、142 万セル、一様流からの開始、300 反復）。左は密度の残差、右は :math:`\alpha=4^\circ` の揚力係数。残差が下がっても係数が落ち着くまで反復が続くことを見る。最初の 15 反復ほどの大きな振れは縦軸の範囲の外である。

5. 中断した計算を再開する
~~~~~~~~~~~~~~~~~~~~~~~~~~

途中で止めた（Ctrl-C、SIGTERM、時間切れ）あと、同じ設定で ``ignisyeet aero`` をもう一度実行すると、終わったケースは再利用され、残りだけが解かれる。
最初からやり直すには、出力先の下の ``cfd`` ディレクトリ（``workdir`` で変えられる）を削除する。

6. RANS で解く（摩擦抗力も CFD から得る場合）
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

Euler の設定を ``model = "rans"`` に替えるだけで始められるが、``examples/sample_rans.toml`` を出発点にするほうが確実である。

.. code-block:: console

   $ ignisyeet cfd-check examples/sample_rans.toml --mesh
   $ ignisyeet aero examples/sample_rans.toml

* ``wall_size``、``iterations`` などを設定しないと、RANS 用の既定値（壁面 5 mm、``yplus`` 1）が使われる。
* 使える形状と設定は限られる。``surface = "stl"``、``y=0`` について対称でないフィン配置の ``panel_mesh``、``roll_damping = true``、``yplus`` が 5 より大きく 30 より小さい場合は、計算を始める前にエラーで止まる。
  条件と理由は :ref:`sec-cfd-rans-use` にある。
* 1 ケースのメモリは Euler よりずっと大きい。サンプル機体の半モデルで ``wall_size`` 5 mm のとき、``cfd-check --mesh`` の見積もりは約 9.0 GB（73.5 万節点）で、``wall_size`` 4.8 mm（78.7 万節点）の実測のピークは 7804 MiB だった。3 mm では見積もりが 22.3 GB になる（``doc/data/cfd/rans_mesh_sweep.csv``）。
  4 mm より細かくすると、メモリの目安を示す警告が出る。
* ``sample_rans.toml`` の 20 ケース（:math:`M=0.3, 0.8, 1.2, 2`、:math:`\alpha=0, \pm4, \pm8^\circ`）を 4 ランク、2 ケースずつ同時に解くと約 4.4 時間かかり、15 ケースが ``converged``、5 ケースが ``unconverged`` だった（``doc/data/cfd/rans_table_run.json``。測定の条件は同ファイルの ``machine`` 欄）。
  ``unconverged`` が出たら、実行の最後の警告 ``warning[IY-F508]`` と、``cfd_cases.csv`` の ``rans_verdict`` と ``residual_drop`` を読む（:ref:`IY-F508`）。
* 係数表に使う前に、RANS の検証の範囲を確かめる（:ref:`page-vv-rans-verification`）。

.. _sec-cfd-roll-cases:

7. ロール減衰を CFD で求める（任意）
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

``[aero.cfd]`` に ``roll_damping = true`` を加えると、各 Mach 数で :math:`\alpha=\beta=0` のまま、ロール角速度 :math:`\hat p=\pm` ``roll_rate_hat``\ （既定 0.01）の 2 ケースが追加される。
機体が回る流れとして全領域で解くので、半領域のメッシュは使えず、Euler だけで使える。
2 ケースの差からロール減衰微係数 :math:`C_{lp}` が求まり、係数表に入る。
2 ケースの片方が失敗した Mach 数など、CFD の値を採用できない Mach 数は、部品積み上げ法の値に替わる。採用した側は ``cfd_report.json`` の ``clp_source`` で分かる。
指定しないときの :math:`C_{lp}` は部品積み上げ法の値である。

うまくいかないとき
--------------------

.. list-table::
   :header-rows: 1
   :widths: 42 58

   * - 症状
     - 対処
   * - ``error[IY-F002]: a single CFD case needs about … GB …``
     - ``wall_size`` を大きくするか ``ranks_per_case`` を減らす。メモリを空けられるときは ``resources.memory_gb`` を指定する（:ref:`IY-F002`）
   * - ``error[IY-F120]``、``error[IY-F121]``\ （SU2 がシグナルで止められた）
     - 多くはメモリ不足である。``parallel_cases`` か ``ranks_per_case`` を減らす（:ref:`IY-F120`、:ref:`IY-F121`）
   * - ``error[IY-F118]``、``error[IY-F119]``
     - ケースが発散した、または収束しなかった。メッセージの理由を読む。平らな底面の Euler は収束しないことがある（:ref:`IY-F118`、:ref:`IY-F119`）
   * - ``error[IY-F004]``
     - 最小または最大の Mach 数のケースがすべて失敗し、表を補間できない。そのケースの失敗の理由を直す（:ref:`IY-F004`）
   * - ``warning[IY-F508]``
     - RANS のケースが残差の停滞のまま受け入れられた。係数のばらつきを確かめる（:ref:`IY-F508`）
   * - ``warning[IY-F509]``
     - :math:`\alpha=\beta=0` に横力が出ている。対称な機体なら、形状かメッシュの非対称が原因である（:ref:`IY-F509`）
   * - 終了ステータス 1 とツールの不足
     - :ref:`page-howto-cfd-setup`

関連
------

* 設定キー：:ref:`page-reference-config-cfd`
* 出力ファイルの列と項目：:ref:`page-reference-cfd-output-files`
* CFD の選択肢（Euler と RANS、壁面、対称性、メッシュ、``alpha_mode``、資源）の決め方：:ref:`page-theory-cfd-choose`
* 方程式と離散化：:ref:`page-theory-cfd-equations`
* 係数表の作り方（数式）：:ref:`page-theory-cfd-coefficients`
* CAD のモデルから作る：:ref:`page-howto-cad-step`
* 精度の根拠と計算時間：:ref:`page-vv-cfd-verification`、:ref:`page-vv-rans-verification`、:ref:`page-vv-performance`
