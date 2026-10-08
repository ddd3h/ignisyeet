.. _page-howto-cfd-setup:

CFD のツールを用意する
========================

CFD で係数表を作る（``aero.method = "cfd"``）ために必要な外部ツールを入れ、使える状態であることを確かめる手順を示す。
部品積み上げ法とパネル法だけを使うときは、このページは要らない。

前提
------

* ``ignisyeet`` がインストールされている（:ref:`page-howto-install`）。
* Linux（x86_64 または aarch64）か macOS で、ネットワークに接続できる。
* ディスクに約 3 GB の空きがある（conda-forge のパッケージ。Linux でソースから SU2 を構築するときは、さらに構築中だけ約 0.5 GB）。

必要なツールは次のとおりである。

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
     - 1 ケースを複数のランクで並列に解くとき（``ranks_per_case`` が 2 以上）に必要 :cite:`openmpi`
   * - manifold3d（Python。3.5.4）
     - ``surface = "stl"`` のときだけ必要 :cite:`manifold`

SU2 は 8.3.0 に固定されている（:ref:`IY-I510`）。

手順
------

1. **インストーラで CFD ツールを入れる。** 新しく入れるときは ``--cfd`` を付ける。

   .. code-block:: console

      $ curl -fsSL https://raw.githubusercontent.com/ddd3h/ignisyeet/main/install.sh | bash -s -- --cfd

   すでに ``ignisyeet`` を入れてあるときも、同じコマンドで CFD ツールを追加できる。
   対話式の質問に答えて入れる場合は、インストーラの CFD ツールの質問に ``y`` と答える（:ref:`page-howto-install`）。
   実行前に内容だけ確かめるときは ``--dry-run`` を付ける。

   期待する結果： 最後に、設定ファイルの ``[aero.cfd] prefix`` に書く値（または環境変数 ``IGNISYEET_CFD_PREFIX`` に入れる値）が表示される。
   Linux では、続けて SU2 のその CPU 向けの構築が行われる（下の「SU2 の構築」）。

2. **CFD 環境の場所を ``ignisyeet`` に教える。** 設定ファイルに書くか、環境変数で渡す。

   .. code-block:: toml

      [aero.cfd]
      prefix = "~/.local/share/micromamba/envs/ignisyeet-cfd"

   .. code-block:: console

      $ export IGNISYEET_CFD_PREFIX=~/.local/share/micromamba/envs/ignisyeet-cfd

   ``prefix`` が空のときは ``IGNISYEET_CFD_PREFIX`` が使われ、それも空のときは ``PATH`` から探される。設定の先頭の ``~`` はホームディレクトリに展開される。

3. **ツールが使えることを確かめる。**

   .. code-block:: console

      $ ignisyeet cfd-check examples/sample_cfd.toml

   期待する結果： 4 つのツールの場所とバージョンが ``ok`` で並び、最後に計算ケース数が出る。

   .. code-block:: text

      CFD tools for examples/sample_cfd.toml
        prefix      /home/<user>/.local/share/micromamba/envs/ignisyeet-cfd
        SU2_CFD              ok       SU2 v8.3.0 "Harrier", The Open-Source CFD Code  [/home/<user>/.local/share/ignisyeet-su2-native/bin/SU2_CFD]
        mpirun               ok       mpirun (Open MPI) 5.0.11  [.../envs/ignisyeet-cfd/bin/mpirun]
        gmsh (python)        ok       gmsh 4.15.2  [.../envs/ignisyeet-cfd/bin/python]
        manifold3d (python)  ok       manifold3d installed  [.../envs/ignisyeet-cfd/bin/python]
      CFD mode is ready (12 cases for model Euler).

   manifold3d の行は ``surface = "stl"`` のときだけ検査される。
   ``surface = "stl"`` か ``"step"`` で ``shape_symmetry`` が ``"auto"`` のときは、対称性が形状を調べるまで分からないので、ケース数は ``up to 20 cases`` のように全領域を解く場合の上限で示される。

4. **メッシュを確かめる（任意）。** ``--mesh`` を付けると、実際にメッシュを作って品質を表示する。

   .. code-block:: console

      $ ignisyeet cfd-check examples/sample_step.toml --mesh

   期待する結果： 節点数、セル数、壁面の三角形数、セルの種類、品質（SICN の最小値と平均、0.05 未満のセル数）が出る。
   ``examples/sample_step.toml`` の例では、節点数 560092、セル数 2690207、壁面の三角形数 479830、SICN の最小値 0.071、平均 0.783、0.05 未満のセル 0 で、メッシュ生成に 101 秒かかった（20 スレッドのマシン）。
   ``surface = "stl"`` では形状の忠実度、``surface = "step"`` では CAD のモデルの読み込みと位置合わせの結果も出る（:ref:`page-howto-cad-step`）。
   作ったメッシュは本番の計算でそのまま再利用される。

確認
------

* 手順 3 の最終行が ``CFD mode is ready`` である。
* 終了ステータスが 0 である。ツールが足りないときは、不足を報告して終了ステータス 1 で終わる。
* CFD モードで ``aero`` を実行したときも、計算を始める前に同じ検査が行われる。

.. _sec-cfd-native:

SU2 の構築（Linux）
---------------------

Linux では ``--cfd`` により、SU2 8.3.0 がソースから、その場の CPU 向けに構築される（所要時間は :ref:`page-howto-install` の C）。
構築を省いて移植できる conda-forge のバイナリを使うときは、``--su2-binary``\ （環境変数 ``IGNISYEET_SU2_BINARY=1``）を付ける。macOS では常にバイナリが使われる。
どちらを選ぶかと、速さの違いは :ref:`sec-vperf-su2-native` にある。
使われる SU2 の選ばれ方と環境変数は :ref:`page-reference-installer` にあり、どの SU2 が選ばれたかは ``cfd-check`` が表示する ``SU2_CFD`` のパスで分かる。

手作業で入れる場合
--------------------

インストーラを使わず、micromamba で同じ環境を作ることもできる。

.. code-block:: console

   $ micromamba create -n ignisyeet-cfd -c conda-forge python=3.10 su2=8.3.0 gmsh=4.15.2 python-gmsh=4.15.2 openmpi=5
   $ ~/.local/share/micromamba/envs/ignisyeet-cfd/bin/python -m pip install manifold3d==3.5.4

manifold3d は ``surface = "stl"`` のときだけ要る。この場合の SU2 は conda-forge のバイナリである。

うまくいかないとき
--------------------

.. list-table::
   :header-rows: 1
   :widths: 42 58

   * - 症状
     - 対処
   * - ``error[IY-F001]: SU2 is not installed …``
     - 手順 1 で ``--cfd`` を付けて入れ、手順 2 で ``prefix`` を合わせる（:ref:`IY-F001`）
   * - ``error[IY-F007]: the python module manifold3d is missing …``
     - ``<python> -m pip install manifold3d`` を実行する。``surface = "stl"`` のときだけ要る（:ref:`IY-F007`）
   * - ``warning[IY-F501]: SU2 … found; the documentation and validation are pinned to SU2 8.3.0``
     - SU2 を 8.3.x にする。他の版でも動くが、結果が変わりうる（:ref:`IY-F501`）
   * - ``warning[IY-F503]: <SU2_CFD> was compiled for the CPU …``
     - 別の CPU に移した構築済みの SU2 である。その CPU 上で ``install.sh --cfd --force`` を実行して作り直すか、``--su2-binary`` に切り替える（:ref:`IY-F503`）
   * - ``warning[IY-I540]: SU2 could not be compiled: …``
     - ネットワークの接続かコンパイルのエラーによる。conda-forge の SU2 がそのまま使われる。``install.sh --cfd --force`` で再試行するか、``--su2-binary`` を選ぶ（:ref:`IY-I540`）
   * - ``warning[IY-I531]``、``warning[IY-I532]``
     - CFD 環境を作れない、または環境に SU2 がない。メッセージの案内に従う（:ref:`IY-I531`、:ref:`IY-I532`）
   * - ``warning[IY-I533]: manifold3d could not be installed …``
     - 表示された再試行のコマンドを実行する（:ref:`IY-I533`）
   * - ``warning[IY-I510]: su2 … needs AVX-512, which this CPU lacks``
     - ``su2=8.3.0`` で環境を作り直す（:ref:`IY-I510`）

``--su2-binary`` を付けて実行すると、すでにある ``share/ignisyeet-su2-native`` は削除される。
``--uninstall`` はこのディレクトリを削除するか尋ね、``--purge`` を付けると尋ねずに削除する。

関連
------

* インストーラのオプションと環境変数：:ref:`page-reference-installer`
* CFD の設定キー：:ref:`page-reference-config-cfd`
* 係数表を作る：:ref:`page-howto-cfd-table`
* CAD のモデルから作る：:ref:`page-howto-cad-step`
* CFD の選択肢の決め方：:ref:`page-theory-cfd-choose`
* 計算時間とメモリの測定：:ref:`page-vv-performance`
