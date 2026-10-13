.. _errors-cfd:

CFD のエラーとワーニング
========================

CFD（SU2 によるテーブル作成、``cfd-check``）のエラーとワーニングのコードである。コードは ``IY-F`` に 3 桁の番号が続く形で、エラーは 001 から 499、ワーニングは 500 から 999 である。
メッシャーのスクリプトが出すエラーはログの末尾に ``[IY-F0nn] mesh error: ...`` の形で現れ、:ref:`IY-F010` のコードは付かない。
画面には ``error[IY-F118]: aero.cfd: case m0.500_a00.00 failed: …`` や ``warning[IY-F523]: aero.cfd: case m0.500_a00.00: …`` のように、コードの後ろに ``aero.cfd:`` とケース名が続く形で出る。
``cfd_cases.csv`` と ``cfd_report.json`` などの機械出力の文字列には、``[IY-F118] …`` のままコードを先頭に付けて書く。
CFD の設定値の検査で出るコード（``IY-C``）は設定のエラー一覧にある。

.. _IY-F001:

IY-F001
-------

重大度: エラー

メッセージ::

    error[IY-F001]: SU2 is not installed (install with `install.sh --cfd`, see the CFD chapter of the documentation)
      - <tool>: <problem>

意味
    CFD に必要なツール（SU2_CFD、ランク数が 2 以上のときの mpirun、gmsh を含む Python、surface = "stl" のときの manifold3d）のいずれかが見つからないか、動作しない。各行にツール名と原因が並ぶ。

主な原因
    CFD 用環境を導入していない。IGNISYEET_CFD_PREFIX、aero.cfd.prefix、aero.cfd.su2 が実際の場所を指していない。SU2 のバイナリが実行できない。

対処
    ``install.sh --cfd`` で導入するか、``IGNISYEET_CFD_PREFIX`` を導入先に合わせる。``ignisyeet cfd-check`` で各ツールの状態を確認できる。

関連
    :ref:`sec-cfd-native`

.. _IY-F002:

IY-F002
-------

重大度: エラー

メッセージ::

    error[IY-F002]: a single CFD case needs about <x> GB (<Euler|RANS> on <cells> cells / <nodes> nodes, <n> ranks), more than the memory budget of <y> GB (<source>); use a coarser mesh (a larger wall_size), fewer ranks_per_case (about 40 MB each), or raise resources.memory_gb

意味
    メッシュの規模から見積もった 1 ケース分のメモリが、使えるメモリ予算を超えている。1 ケースも実行できないため計算を始めない。メッセージ本文は ``aero.cfd:`` と空白の後ろに表示される。

主な原因
    wall_size が小さすぎる。RANS のメッシュが大きい。ranks_per_case が多い。他のプログラムがメモリを使っていて空きが少ない。

対処
    wall_size を大きくする（RANS では既定値 0.005 m を目安にする）か、ranks_per_case を減らす。メモリを空けられるときは resources.memory_gb を指定する。

関連
    :ref:`[resources] の資源指定 <sec-cfd-resources>`

.. _IY-F003:

IY-F003
-------

重大度: エラー

メッセージ::

    error[IY-F003]: no CFD cases to build a table from

意味
    空気力テーブルを組み立てる CFD ケースが 1 つもない。

主な原因
    aero.cfd のマッハ数または迎角の指定が空になっている。

対処
    aero.cfd.machs と alphas_deg に値を指定する。

関連
    :ref:`sec-cfd-table-build`

.. _IY-F004:

IY-F004
-------

重大度: エラー

メッセージ::

    error[IY-F004]: all CFD cases at the <lowest|highest> Mach number (<M>) failed (or have no angle of attack above 0 | have no usable angles); the table cannot be bridged there

意味
    テーブルの端にあたるマッハ数で、収束したケースが 1 つもない。端の列は内挿で埋められないためテーブルを作れない。

主な原因
    そのマッハ数のケースがすべて発散・未収束・打ち切りになった。有効な迎角が 0 度以下しか残っていない。

対処
    cfd_cases.csv と各ケースの su2.log で失敗理由（F110 から F123）を確認し、メッシュや反復数を見直す。端のマッハ数を計算範囲から外す方法もある。

関連
    :ref:`sec-cfd-table-build`

.. _IY-F005:

IY-F005
-------

重大度: エラー

メッセージ::

    error[IY-F005]: surface = "stl" needs the STL triangles

意味
    surface = "stl" の CFD に STL の三角形が渡されていない。

主な原因
    形状が STL ではなくパラメータ記述から作られている。

対処
    geometry.source を STL にするか、surface を panel_mesh にする。

関連
    :ref:`sec-cfd-stl`

.. _IY-F006:

IY-F006
-------

重大度: エラー

メッセージ::

    error[IY-F006]: surface = "step" needs the STL triangles (the registration reference)

意味
    surface = "step" では、STEP との位置合わせの基準になる STL が必要である。

主な原因
    形状が STL から読み込まれていない。

対処
    STL から形状を読み込む設定にする。

関連
    :ref:`sec-cfd-step`

.. _IY-F007:

IY-F007
-------

重大度: エラー

メッセージ::

    error[IY-F007]: the python module manifold3d is missing (needed for surface = "stl"); install it with: <python> -m pip install manifold3d

意味
    surface = "stl" の部品の和集合に使う manifold3d が、gmsh を含む Python にない。

主な原因
    manifold3d を導入していない環境である。

対処
    表示された ``pip install`` を実行するか、``install.sh --cfd`` を再実行する。

関連
    :ref:`sec-cfd-stl`、:ref:`sec-cfd-native`

.. _IY-F010:

IY-F010
-------

重大度: エラー

メッセージ::

    error[IY-F010]: <what> failed (log: <path>):
    <last lines of the log>

意味
    メッシャー（gmsh を呼ぶ Python）が異常終了した。スクリプト自身のコード付きエラーがログの末尾にあるときは、このコードは付かずそちらのコードだけが表示される。

主な原因
    gmsh の内部エラー。入力形状またはメッシュ設定が gmsh の扱える範囲を超えた。

対処
    ログの末尾を読み、wall_size や形状を見直す。表示された log のパスに全文がある。

関連
    :ref:`sec-cfd-stl`、:ref:`sec-cfd-step`

.. _IY-F011:

IY-F011
-------

重大度: エラー

メッセージ::

    error[IY-F011]: <what> failed (killed by signal <n>[ (most likely out of memory: close other programs or use a larger wall_size)]) (log: <path>):
    <last lines of the log>

意味
    メッシャーがシグナルで強制終了された。シグナル 9（SIGKILL）は多くの場合メモリ不足による強制終了である。

主な原因
    メモリ不足。メッシュが細かすぎる。

対処
    他のプログラムを閉じるか、wall_size を大きくする。

関連
    :ref:`[resources] の資源指定 <sec-cfd-resources>`

.. _IY-F012:

IY-F012
-------

重大度: エラー

メッセージ::

    error[IY-F012]: aero.cfd: the RANS boundary-layer mesh needs a fin layout symmetric about the y = 0 plane (fin_roll_deg = <deg> with <n> fins is not): use a symmetric fin_roll_deg (for example 0 or 45 with 4 fins) or model = "euler"

意味
    RANS の境界層メッシュは半断面で作るため、フィン配置が y = 0 面について対称である必要がある。

主な原因
    fin_roll_deg がフィン配置を y = 0 に対して非対称にしている。

対処
    対称になる fin_roll_deg にするか、model = "euler" にする。

関連
    :ref:`sec-cfd-prism`、:ref:`sec-cfd-domain`

.. _IY-F020:

IY-F020
-------

重大度: エラー

メッセージ::

    error[IY-F020]: mesh error: the prismatic boundary layer needs the half model (a fin layout symmetric about y = 0) without z_mirror

意味
    プリズム境界層は半断面モデルでのみ作れる。

主な原因
    全域メッシュまたは z_mirror のメッシュが指定されている。

対処
    対称なフィン配置の半断面メッシュにするか、model = "euler" にする。

関連
    :ref:`sec-cfd-prism`

.. _IY-F021:

IY-F021
-------

重大度: エラー

メッセージ::

    error[IY-F021]: mesh error: boolean operations produced <n> volumes (expected 1); the geometry may be invalid

意味
    計算領域を作るブーリアン演算の結果が、1 つの体積にならなかった。

主な原因
    形状が閉じていない。部品が離れている、または重なりが不正である。

対処
    形状を修正して閉じた 1 つの立体にする。

関連
    :ref:`sec-cfd-domain`

.. _IY-F022:

IY-F022
-------

重大度: エラー

メッセージ::

    error[IY-F022]: mesh error: could not classify the boundary faces[ (wall <n>, base <n>, cut <n>): the shape needs aft-facing faces at its base x = <x> m | facets]

意味
    境界面を壁・底面・対称面などに分類できなかった。

主な原因
    底面が x = 機体長さ の平面になっていない。形状の座標系が想定とずれている。

対処
    機体が x = 機体長さの平らな底面で終わる形状にする。

関連
    :ref:`sec-cfd-stl-base`

.. _IY-F023:

IY-F023
-------

重大度: エラー

メッセージ::

    error[IY-F023]: mesh error: the volume mesh is empty (3D meshing failed)

意味
    体積メッシュに要素が 1 つもできなかった。

主な原因
    表面メッシュが不正で gmsh が体積を作れなかった。

対処
    wall_size を変えるか、形状を修正する。

関連
    :ref:`sec-cfd-domain`

.. _IY-F024:

IY-F024
-------

重大度: エラー

メッセージ::

    error[IY-F024]: mesh error: 3D meshing failed (algorithm <n>): <gmsh message>

意味
    gmsh の 3 次元メッシングが失敗した。HXT で失敗した Euler メッシュは Delaunay で 1 度やり直す。

主な原因
    品質の悪い表面メッシュ。鋭い先端や極端に小さな面。

対処
    stl_remesh = "none" や wall_size を試す。

関連
    :ref:`sec-cfd-stl`、:ref:`sec-cfd-step`

.. _IY-F030:

IY-F030
-------

重大度: エラー

メッセージ::

    error[IY-F030]: mesh error: the wall surface mesh is not manifold (<n> edges with more than two triangles)

意味
    境界層を作る壁の表面メッシュに、3 枚以上の三角形が共有する辺がある。

主な原因
    壁が自己交差している。

対処
    形状を修正する。

関連
    :ref:`sec-cfd-prism`

.. _IY-F031:

IY-F031
-------

重大度: エラー

メッセージ::

    error[IY-F031]: mesh error: the wall surface mesh is not orientable

意味
    壁の表面メッシュの向きを揃えられない。

主な原因
    メビウスの帯のような不正な接続がある。

対処
    形状を修正する。

関連
    :ref:`sec-cfd-prism`

.. _IY-F032:

IY-F032
-------

重大度: エラー

メッセージ::

    error[IY-F032]: mesh error: the wall surface mesh has non-triangular elements

意味
    壁の表面メッシュに三角形以外の要素がある。

主な原因
    gmsh が四角形などを出力した。

対処
    wall_size を変えて再実行する。

関連
    :ref:`sec-cfd-prism`

.. _IY-F033:

IY-F033
-------

重大度: エラー

メッセージ::

    error[IY-F033]: mesh error: the wall has <n> open edges (the shape is not closed)

意味
    壁面に開いた辺があり、閉じた立体になっていない。

主な原因
    形状に穴や隙間がある。

対処
    形状を閉じた立体に修正する。

関連
    :ref:`sec-cfd-prism`

.. _IY-F034:

IY-F034
-------

重大度: エラー

メッセージ::

    error[IY-F034]: mesh error: the wall has an open edge away from the symmetry plane

意味
    対称面以外の場所に開いた辺がある。

主な原因
    半断面に切った壁面が対称面以外で開いている。

対処
    形状を修正するか shape_symmetry = "none" にする。

関連
    :ref:`sec-cfd-prism`、:ref:`sec-cfd-stl-symmetry`

.. _IY-F035:

IY-F035
-------

重大度: エラー

メッセージ::

    error[IY-F035]: mesh error: unexpected element type <n> on the outer surfaces | outside the boundary layer

意味
    境界層メッシュの処理が想定外の要素種別に出会った。

主な原因
    gmsh の出力が想定と違う（内部エラー）。

対処
    ログを添えて報告する。

関連
    :ref:`sec-cfd-prism`

.. _IY-F036:

IY-F036
-------

重大度: エラー

メッセージ::

    error[IY-F036]: mesh error: the tetrahedral mesh outside the boundary layer failed (<gmsh message>)<hint>

意味
    境界層の外側の四面体メッシュが作れなかった。

主な原因
    境界層の上面が品質の悪い表面になっている。

対処
    wall_size や yplus を見直す。

関連
    :ref:`sec-cfd-prism`

.. _IY-F037:

IY-F037
-------

重大度: エラー

メッセージ::

    error[IY-F037]: mesh error: a tetrahedron uses an unknown node

意味
    生成したメッシュに、存在しない節点を指す要素がある。

主な原因
    ソフトウェアの誤り。

対処
    ログと設定ファイルを添えて報告する。

関連
    :ref:`sec-cfd-prism`

.. _IY-F038:

IY-F038
-------

重大度: エラー

メッセージ::

    error[IY-F038]: mesh error: the prismatic boundary layer is invalid: <inverted prisms, non-positive cells, ...><hint>

意味
    プリズム境界層に反転した要素などが残った。平滑化を強めた再試行（BL_ATTEMPTS）でも直らなかった場合に出る。

主な原因
    層が厚すぎる。凹角や薄いフィンで法線が交差している。

対処
    yplus を上げる、wall_size を大きくする、形状の鋭い凹角を緩める。

関連
    :ref:`sec-cfd-prism`

.. _IY-F039:

IY-F039
-------

重大度: エラー

メッセージ::

    error[IY-F039]: mesh error: the volume mesh is invalid: <n> cells of non-positive volume, <n> boundary faces without marker, <n> marker faces inside

意味
    書き出す前の体積メッシュの検査で不正が見つかった。

主な原因
    gmsh が不正な要素を出した。

対処
    wall_size を変えて再実行する。

関連
    :ref:`sec-cfd-step`

.. _IY-F040:

IY-F040
-------

重大度: エラー

メッセージ::

    error[IY-F040]: mesh error: <file> contains no closed solid (<n> faces could not be sewn into one); export the rocket as a solid body

意味
    CAD ファイルに閉じたソリッドがない。

主な原因
    サーフェスだけで書き出している。面が縫い合わされていない。

対処
    CAD でソリッド体として書き出し直す。

関連
    :ref:`sec-cfd-step`

.. _IY-F041:

IY-F041
-------

重大度: エラー

メッセージ::

    error[IY-F041]: mesh error: gmsh cannot mesh the surface of <file>, even after shape healing (<detail>); repair the CAD model or export it again

意味
    形状修復の後でも CAD 面の表面メッシュを作れない。

主な原因
    CAD モデルの面が壊れている。

対処
    CAD モデルを修復するか書き出し直す。

関連
    :ref:`sec-cfd-step`

.. _IY-F042:

IY-F042
-------

重大度: エラー

メッセージ::

    error[IY-F042]: mesh error: unknown mode <mode>

意味
    メッシュ生成が未対応の動作モードを受け取った。

主な原因
    ソフトウェアの誤り。

対処
    ログと設定ファイルを添えて報告する。

.. _IY-F043:

IY-F043
-------

重大度: エラー

メッセージ::

    error[IY-F043]: mesh error: the prepared shape has no solid

意味
    準備した形状にソリッドが残っていない。

主な原因
    CAD の読み込み・融合で形状が消えた。

対処
    CAD ファイルの内容を確認する。

関連
    :ref:`sec-cfd-step`

.. _IY-F044:

IY-F044
-------

重大度: エラー

メッセージ::

    error[IY-F044]: mesh error: the surface mesh of the CAD faces <faces> has duplicated or degenerate triangles with every surface algorithm

意味
    どの表面アルゴリズムでも、示した CAD 面のメッシュに重複または退化した三角形が残る。

主な原因
    CAD 面の形状が壊れている。

対処
    該当する面を CAD で修復する。

関連
    :ref:`sec-cfd-step`

.. _IY-F045:

IY-F045
-------

重大度: エラー

メッセージ::

    error[IY-F045]: mesh error: the prismatic boundary layer is not built on the quarter domain

意味
    プリズム境界層は 1/4 領域では作れない。

主な原因
    RANS で 1/4 領域のメッシュが選ばれた。通常は選ばれない組み合わせで、ソフトウェアの誤りである。

対処
    ログと設定ファイルを添えて報告する。

関連
    :ref:`sec-cfd-prism`

.. _IY-F046:

IY-F046
-------

重大度: エラー

メッセージ::

    error[IY-F046]: mesh error: the quarter domain cannot be cut from the shape (no wall faces left); QUARTER_FAILED

意味
    形状から 1/4 領域を切り出せない。通常は半断面メッシュに切り替えて続行する。

主な原因
    OpenCASCADE のブーリアン演算が失敗した。

対処
    自動で半断面にやり直す。繰り返す場合は shape_symmetry を見直す。

関連
    :ref:`sec-cfd-stl-symmetry`

.. _IY-F047:

IY-F047
-------

重大度: エラー

メッセージ::

    error[IY-F047]: mesh error: no base faces (aft-facing faces at the base x = <x> m) found; the shape must end in a flat base or use base_treatment = "fairing"

意味
    底面（x = 機体長さ にある後ろ向きの面）が見つからない。

主な原因
    形状が平らな底面で終わっていない。

対処
    平らな底面で終わる形状にするか base_treatment = "fairing" を使う。

関連
    :ref:`sec-cfd-stl-base`

.. _IY-F050:

IY-F050
-------

重大度: エラー

メッセージ::

    error[IY-F050]: mesh error: stl_remesh must be classify or none, got <value>

意味
    ``aero.cfd.stl_remesh`` の値が不正である。

主な原因
    値が ``"classify"`` でも ``"none"`` でもない。

対処
    ``aero.cfd.stl_remesh`` を ``"classify"`` か ``"none"`` にする。

関連
    :ref:`sec-cfd-stl`

.. _IY-F051:

IY-F051
-------

重大度: エラー

メッセージ::

    error[IY-F051]: mesh error: gmsh cannot parametrise the united surface (<detail>). Sharp apexes and facets of very poor quality do that; stl_remesh = "none" meshes the triangles as they are (the fins are then under-resolved).

意味
    gmsh が和集合の表面を再パラメータ化できない。

主な原因
    鋭い頂点や品質の極端に悪い三角形がある。

対処
    stl_remesh = "none" にする。フィンの分解能は下がる。

関連
    :ref:`sec-cfd-stl`、:ref:`sec-cfd-fidelity`

.. _IY-F052:

IY-F052
-------

重大度: エラー

メッセージ::

    error[IY-F052]: mesh error: the STL surface has no triangles

意味
    STL の表面に三角形がない。

主な原因
    STL が空、または前処理で全部除かれた。

対処
    STL を確認する。

関連
    :ref:`sec-cfd-stl`

.. _IY-F053:

IY-F053
-------

重大度: エラー

メッセージ::

    error[IY-F053]: mesh error: a boundary curve of the clipped surface (bounding box <bb>) lies in neither or both symmetry planes

意味
    対称面で切った表面の境界曲線が、どちらの対称面にも属さないか、両方に属している。

主な原因
    形状が対称面できれいに切れていない。

対処
    shape_symmetry = "none" にする。

関連
    :ref:`sec-cfd-stl-symmetry`

.. _IY-F054:

IY-F054
-------

重大度: エラー

メッセージ::

    error[IY-F054]: mesh error: the cut of the clipped surface is not a closed curve

意味
    対称面での切り口が閉曲線になっていない。

主な原因
    形状が対称面できれいに切れていない。

対処
    shape_symmetry = "none" にする。

関連
    :ref:`sec-cfd-stl-symmetry`

.. _IY-F055:

IY-F055
-------

重大度: エラー

メッセージ::

    error[IY-F055]: mesh error: the cut of the clipped surface is not one open curve between two points on the axis (end points <points>)

意味
    切り口が、軸上の 2 点を結ぶ 1 本の開曲線になっていない。

主な原因
    形状が軸対称・面対称から外れている。

対処
    shape_symmetry = "none" にする。

関連
    :ref:`sec-cfd-stl-symmetry`

.. _IY-F056:

IY-F056
-------

重大度: エラー

メッセージ::

    error[IY-F056]: mesh error: broken cut curve

意味
    切り口の曲線が途切れている。

主な原因
    形状が対称面できれいに切れていない。

対処
    shape_symmetry = "none" にする。

関連
    :ref:`sec-cfd-stl-symmetry`

.. _IY-F057:

IY-F057
-------

重大度: エラー

メッセージ::

    error[IY-F057]: mesh error: unexpected cut curves of the clipped surface (y = 0: <n>, z = 0: <n>)

意味
    切り口の曲線の本数が想定と違う。

主な原因
    形状が対称面できれいに切れていない。

対処
    shape_symmetry = "none" にする。

関連
    :ref:`sec-cfd-stl-symmetry`

.. _IY-F058:

IY-F058
-------

重大度: エラー

メッセージ::

    error[IY-F058]: mesh error: the cuts at y = 0 and z = 0 do not meet at the same points of the axis

意味
    y = 0 と z = 0 の切り口が軸上の同じ点で交わらない。

主な原因
    形状が軸対称から外れている。

対処
    shape_symmetry = "none" か "pitch_plane" にする。

関連
    :ref:`sec-cfd-stl-symmetry`

.. _IY-F059:

IY-F059
-------

重大度: エラー

メッセージ::

    error[IY-F059]: mesh error: no base facets (aft-facing facets at x = length) found; the STL must end in a flat base or use base_treatment = fairing

意味
    STL に底面の三角形が見つからない。

主な原因
    STL が平らな底面で終わっていない。

対処
    平らな底面で終わる STL にするか base_treatment = "fairing" を使う。

関連
    :ref:`sec-cfd-stl-base`

.. _IY-F060:

IY-F060
-------

重大度: エラー

メッセージ::

    error[IY-F060]: union error: component <i> (<file>) is not a valid manifold: <status>

意味
    和集合を取る前の検査で、STL の部品が有効な多様体ではない。

主な原因
    STL の部品に穴や自己交差がある。

対処
    STL を修復する。

関連
    :ref:`sec-cfd-stl`

.. _IY-F061:

IY-F061
-------

重大度: エラー

メッセージ::

    error[IY-F061]: union error: component <i> has a non-positive volume <v> (inside-out?)

意味
    STL の部品の体積が 0 以下である。

主な原因
    面の向きが反転している。

対処
    STL の法線の向きを直す。

関連
    :ref:`sec-cfd-stl`

.. _IY-F062:

IY-F062
-------

重大度: エラー

メッセージ::

    error[IY-F062]: union error: manifold3d returned <status>

意味
    manifold3d の和集合演算がエラーを返した。

主な原因
    部品同士の接触が不正である。

対処
    STL を修復する。

関連
    :ref:`sec-cfd-stl`

.. _IY-F063:

IY-F063
-------

重大度: エラー

メッセージ::

    error[IY-F063]: union error: the united solid has a non-positive volume <v>

意味
    和集合の体積が 0 以下である。

主な原因
    部品の向きが不整合である。

対処
    STL を修復する。

関連
    :ref:`sec-cfd-stl`

.. _IY-F064:

IY-F064
-------

重大度: エラー

メッセージ::

    error[IY-F064]: union error: the united surface consists of several detached bodies (genus <g>): a part of the STL (e.g. a fin) does not touch the rest

意味
    和集合が互いに離れた複数の物体になっている。

主な原因
    フィンなどの部品が本体に接していない。

対処
    部品が本体に接する（食い込む）よう STL を直す。

関連
    :ref:`sec-cfd-stl`

.. _IY-F070:

IY-F070
-------

重大度: エラー

メッセージ::

    error[IY-F070]: aero.cfd.step_unit = "<unit>": the length unit of <file> cannot be read from the file, so it cannot be overridden; use step_unit = "auto"

意味
    ファイルから長さの単位を読めないため step_unit で上書きできない。

主な原因
    単位を持たない形式（IGES など）で step_unit を指定した。

対処
    step_unit = "auto" にする。

関連
    :ref:`sec-cfd-step`

.. _IY-F071:

IY-F071
-------

重大度: エラー

メッセージ::

    error[IY-F071]: aero.cfd.surface = "step": <file> does not describe the same rocket as rocket.stl (compared in the CFD frame, after the transform of the STL):
      - <problem>
      <registration summary>
      likely cause: <cause>

意味
    STEP と STL の位置合わせ検査で、2 つが同じ機体でないと判定された。
    検査は、CFD の座標で STL と CAD を比べ、次の条件をすべて満たすときに同じ機体とみなす。
    :math:`L` は STL から抽出した全長、:math:`h_w` は ``wall_size`` である。

    * STL の包含箱が、CAD の包含箱に :math:`0.01L` の余裕で収まる。
    * ノーズの先端（:math:`x` の最小値）の差が :math:`0.01L` 以下である。
    * CAD の包含箱が、どの軸でも STL の 1.25 倍を超えない。
    * STL の面上の点が、ほぼすべて CAD の面から :math:`\max(2h_w,\,0.01L)` 以内にある。CAD の立体の内側に深く埋まった STL の面（胴体に差し込んだ箱で表したフィンなど）は除く。

    STL の面から :math:`\max(h_w,\,0.002L)` より離れた CAD だけの部分は、細部として許される。
    サンプル機体（:math:`L` = 1.5 m）を ``wall_size`` 2 mm で比べると、許容は 15 mm である。

主な原因
    リビジョンの違う CAD モデル。単位の取り違え。座標系のずれ。

対処
    表示された原因を確認し、STEP と STL を同じモデルから書き出す。

関連
    :ref:`読み込みと位置合わせの報告 <sec-cfd-step-report>`

.. _IY-F072:

IY-F072
-------

重大度: エラー

メッセージ::

    error[IY-F072]: aero.cfd.step_file <file>: not a STEP, IGES or BREP file

意味
    ファイルの形式が STEP・IGES・BREP のどれでもない。

主な原因
    拡張子や内容が違う。

対処
    STEP、IGES、BREP のファイルを指定する。

関連
    :ref:`sec-cfd-step`

.. _IY-F073:

IY-F073
-------

重大度: エラー

メッセージ::

    error[IY-F073]: cannot read the CAD file aero.cfd.step_file = <file>

意味
    CAD ファイルを読み込めない。

主な原因
    パスの誤り。読み取り権限がない。

対処
    step_file のパスと権限を確認する。

関連
    :ref:`sec-cfd-step`

.. _IY-F074:

IY-F074
-------

重大度: エラー

メッセージ::

    error[IY-F074]: aero.cfd.surface = "step": <fairing error>

意味
    base_treatment = "fairing" の円錐を CAD 形状に融合できなかった。

主な原因
    底面付近の CAD 形状が融合に適さない。

対処
    base_treatment = "flat" にするか、底面付近の形状を見直す。

関連
    :ref:`sec-cfd-stl-base`

.. _IY-F075:

IY-F075
-------

重大度: エラー

メッセージ::

    error[IY-F075]: aero.cfd.surface = "step": the shape has no base faces (aft-facing faces at the base x = <x> m of the STL geometry); the CAD model must end in a flat base

意味
    CAD 形状に底面がない。

主な原因
    CAD モデルが平らな底面で終わっていない。

対処
    平らな底面で終わるモデルにする。

関連
    :ref:`sec-cfd-step`

.. _IY-F076:

IY-F076
-------

重大度: エラー

メッセージ::

    error[IY-F076]: shape_symmetry = "axisymmetric" needs a surface that is mirror symmetric about y = 0 and z = 0, but the united STL is not:
      <y = 0 check>
      <z = 0 check>
      use shape_symmetry = "auto" (the planes that pass the check are used) or "none" (full domain)

意味
    shape_symmetry = "axisymmetric" が指定されているが、表面が両方の面について対称ではない（STEP では "the CAD shape is not" と表示される）。

主な原因
    形状が実際に非対称。許容差が小さすぎる。

対処
    shape_symmetry = "auto" か "none" にする。

関連
    :ref:`sec-cfd-stl-symmetry`

.. _IY-F077:

IY-F077
-------

重大度: エラー

メッセージ::

    error[IY-F077]: shape_symmetry = "pitch_plane" needs a surface that is mirror symmetric about y = 0, but the united STL is not:
      <y = 0 check>
      use shape_symmetry = "auto" or "none" (full domain)

意味
    shape_symmetry = "pitch_plane" が指定されているが、表面が y = 0 について対称ではない。

主な原因
    形状が実際に非対称。

対処
    shape_symmetry = "auto" か "none" にする。

関連
    :ref:`sec-cfd-stl-symmetry`

.. _IY-F078:

IY-F078
-------

重大度: エラー

メッセージ::

    error[IY-F078]: the mesh <file> has no wall facets

意味
    メッシュに壁面の要素がない。

主な原因
    メッシュの境界名が想定と違う。

対処
    メッシュを作り直す（work ディレクトリの mesh を消す）。

関連
    :ref:`sec-cfd-fidelity`

.. _IY-F080:

IY-F080
-------

重大度: エラー

メッセージ::

    error[IY-F080]: the wall surface is not watertight: <n> edges belong to only one triangle (holes) and <n> edges to more than two (non-manifold); CFD needs a closed surface

意味
    壁面が閉じていない。STL では先頭に ``STL component <i> (...)`` の説明が付く。

主な原因
    穴や非多様体の辺がある。

対処
    形状を閉じた表面に修復する。

関連
    :ref:`sec-cfd-stl`

.. _IY-F081:

IY-F081
-------

重大度: エラー

メッセージ::

    error[IY-F081]: the STL has no usable triangles

意味
    STL に使える三角形がない。

主な原因
    STL が空、または退化した三角形だけである。

対処
    STL を確認する。

関連
    :ref:`sec-cfd-stl`

.. _IY-F082:

IY-F082
-------

重大度: エラー

メッセージ::

    error[IY-F082]: STL component <i> is not a manifold surface: the edge (<a>, <b>) belongs to <n> facets

意味
    STL の部品に 3 枚以上の面が共有する辺がある。

主な原因
    自己交差や重複した面がある。

対処
    STL を修復する。

関連
    :ref:`sec-cfd-stl`

.. _IY-F083:

IY-F083
-------

重大度: エラー

メッセージ::

    error[IY-F083]: clipping at the symmetry plane left <n> open edges off the plane and <m> non-manifold edges

意味
    対称面で切った表面が閉じない。先頭に "the united STL surface cannot be clipped at the symmetry plane; use shape_symmetry = \"none\"" が付く。

主な原因
    形状が対称面できれいに切れていない。

対処
    shape_symmetry = "none" にする。

関連
    :ref:`sec-cfd-stl-symmetry`

.. _IY-F084:

IY-F084
-------

重大度: エラー

メッセージ::

    error[IY-F084]: STL component <i> (<n> triangles) encloses no volume (<v> m^3)

意味
    STL の部品が体積を囲んでいない。

主な原因
    部品が平らである、または退化している。

対処
    STL を修復する。

関連
    :ref:`sec-cfd-stl`

.. _IY-F086:

IY-F086
-------

重大度: エラー

メッセージ::

    error[IY-F086]: half model: clipping at y = 0 left <n> open edges off the symmetry plane and <m> non-manifold edges; use symmetry = false

意味
    半断面モデルの切断で表面が閉じない。

主な原因
    形状が y = 0 についてきれいに切れていない。

対処
    symmetry = false にする。

関連
    :ref:`sec-cfd-stl-symmetry`

.. _IY-F087:

IY-F087
-------

重大度: エラー

メッセージ::

    error[IY-F087]: surface = "step": the wall is the CAD B-rep, which only the mesher tessellates (see step_mesh)

意味
    ``surface = "step"`` の壁面は、CAD の曲面のままメッシュにするので、三角形の形では取り出せない。

主な原因
    ソフトウェアの誤り。

対処
    ログと設定ファイルを添えて報告する。

関連
    :ref:`sec-cfd-step`

.. _IY-F088:

IY-F088
-------

重大度: エラー

メッセージ::

    error[IY-F088]: STL geometry error <e> m (CFD wall vs union <u> m, wall vs input STL <i> m) exceeds max_geometry_error <m> m; use stl_remesh = "none", a smaller wall_size, or raise max_geometry_error

意味
    CFD 壁面と入力 STL の形状誤差が max_geometry_error を超えた。

主な原因
    wall_size に対して形状が細かすぎる。再メッシュで形が崩れた。

対処
    stl_remesh = "none"、wall_size を小さく、または max_geometry_error を大きくする。

関連
    :ref:`sec-cfd-fidelity`、:ref:`sec-cfd-int-fidelity`

.. _IY-F100:

IY-F100
-------

重大度: エラー

メッセージ::

    error[IY-F100]: empty history file

意味
    SU2 の history.csv が空である。

主な原因
    SU2 が反復を出す前に終了した。

対処
    su2.log を確認する。

関連
    :ref:`sec-cfd-run`

.. _IY-F101:

IY-F101
-------

重大度: エラー

メッセージ::

    error[IY-F101]: history header has no columns

意味
    history.csv の見出し行に列がない。

主な原因
    SU2 の出力が壊れている。

対処
    su2.log を確認し、ケースを消して再実行する。

関連
    :ref:`sec-cfd-run`

.. _IY-F102:

IY-F102
-------

重大度: エラー

メッセージ::

    error[IY-F102]: cannot read <file>: <os error>

意味
    history.csv を読めない。

主な原因
    ファイルがない、権限がない。

対処
    work ディレクトリを確認する。

関連
    :ref:`sec-cfd-run`

.. _IY-F103:

IY-F103
-------

重大度: エラー

メッセージ::

    error[IY-F103]: history has neither CFx nor CD and CL | CFz nor CD and CL

意味
    履歴に力係数の列がない。

主な原因
    SU2 の出力列の設定が想定と違う。

対処
    ログを添えて報告する。

関連
    :ref:`sec-cfd-post`

.. _IY-F104:

IY-F104
-------

重大度: エラー

メッセージ::

    error[IY-F104]: history has no CMy | <column>

意味
    履歴にモーメント係数などの必要な列がない。

主な原因
    SU2 の出力列の設定が想定と違う。

対処
    ログを添えて報告する。

関連
    :ref:`sec-cfd-post`

.. _IY-F105:

IY-F105
-------

重大度: エラー

メッセージ::

    error[IY-F105]: non-finite coefficients in the history (diverged case)

意味
    履歴の係数が有限でない。

主な原因
    ケースが発散した。

対処
    メッシュ・マッハ数・CFL を見直す。

関連
    :ref:`sec-cfd-int-convergence`

.. _IY-F110:

IY-F110
-------

重大度: エラー

メッセージ::

    error[IY-F110]: cannot start SU2: <os error>

意味
    SU2 を起動できない。

主な原因
    実行ファイルまたは mpirun が実行できない。

対処
    ``ignisyeet cfd-check`` で確認する。

関連
    :ref:`sec-cfd-native`

.. _IY-F111:

IY-F111
-------

重大度: エラー

メッセージ::

    error[IY-F111]: interrupted

意味
    利用者の中断（Ctrl-C など）で CFD の実行を止めた。

主な原因
    利用者の操作。

対処
    再実行すると収束済みのケースは再利用される。

関連
    :ref:`sec-cfd-run`

.. _IY-F112:

IY-F112
-------

重大度: エラー

メッセージ::

    error[IY-F112]: the history has no CFx, CFy, CFz, CMx, CMy, CMz columns (needed for sideslip, roll and general tables)

意味
    横滑り・ロール・一般形式のテーブルに必要な 6 分力の列が履歴にない。

主な原因
    SU2 の出力列が想定と違う。

対処
    ログを添えて報告する。

関連
    :ref:`sec-cfd-body6`

.. _IY-F113:

IY-F113
-------

重大度: エラー

メッセージ::

    error[IY-F113]: SU2 produced no history.csv; end of su2.log:
    <last lines>

意味
    SU2 が history.csv を出さずに終わった。

主な原因
    設定ファイルかメッシュの読み込みに失敗した。

対処
    su2.log の末尾を読む。

関連
    :ref:`sec-cfd-run`

.. _IY-F114:

IY-F114
-------

重大度: エラー

メッセージ::

    error[IY-F114]: the first-order startup diverged twice (the second time with conservative settings): <reason>

意味
    RANS の 1 次精度の立ち上げが、保守的な設定でやり直しても発散した。

主な原因
    メッシュ品質が悪い。条件が厳しい。

対処
    メッシュ（wall_size, yplus）を見直し、CFL を下げる。

関連
    :ref:`sec-cfd-int-rans-stages`

.. _IY-F115:

IY-F115
-------

重大度: エラー

メッセージ::

    error[IY-F115]: the first-order startup produced no density and SA residuals; end of su2_startup.log:
    <last lines>

意味
    立ち上げの履歴に密度と SA の残差がない。

主な原因
    SU2 が早期に終了した。

対処
    su2_startup.log を読む。

関連
    :ref:`sec-cfd-int-rans-stages`

.. _IY-F116:

IY-F116
-------

重大度: エラー

メッセージ::

    error[IY-F116]: the first-order startup wrote no <restart file>

意味
    立ち上げがリスタートファイルを書かなかった。

主な原因
    SU2 が書き出す前に終了した。

対処
    su2_startup.log を読む。

関連
    :ref:`sec-cfd-int-rans-stages`

.. _IY-F117:

IY-F117
-------

重大度: エラー

メッセージ::

    error[IY-F117]: the second-order stage diverged with the conservative settings of the repeated first-order startup: <reason>

意味
    2 次精度の段が、保守的に繰り返した立ち上げの後でも発散した。

主な原因
    メッシュ品質が悪い。

対処
    メッシュを見直す。

関連
    :ref:`sec-cfd-int-rans-stages`

.. _IY-F118:

IY-F118
-------

重大度: エラー

メッセージ::

    error[IY-F118]: diverged twice (the second time with conservative settings): <reason>

意味
    ケースが保守的な設定でやり直しても発散した。

主な原因
    メッシュ品質が悪い。

対処
    メッシュを見直し、CFL を下げる。

関連
    :ref:`sec-cfd-int-convergence`、:ref:`sec-cfd-int-rans-stages`

.. _IY-F119:

IY-F119
-------

重大度: エラー

メッセージ::

    error[IY-F119]: not converged (<timeout|iteration limit>): <reason>

意味
    反復上限または時間切れまでに受け入れ基準を満たさなかった。ケースは失敗扱いで、テーブルの穴は周囲から補われる。

主な原因
    反復数が足りない。残差が下がらない。

対処
    iterations や timeout_minutes を増やすか、メッシュを見直す。

関連
    :ref:`sec-cfd-int-convergence`、:ref:`sec-cfd-int-rans-accept`

.. _IY-F120:

IY-F120
-------

重大度: エラー

メッセージ::

    error[IY-F120]: <SU2> was killed by signal 9 (SIGKILL; most likely out of memory: about <x> GB per case with <n> ranks and <m> concurrent cases; lower parallel_cases or ranks_per_case); end of <log>:
    <last lines>

意味
    SU2 が SIGKILL で終了した。多くはメモリ不足である。

主な原因
    同時実行数に対してメモリが足りない。

対処
    parallel_cases または ranks_per_case を減らす。

関連
    :ref:`[resources] の資源指定 <sec-cfd-resources>`

.. _IY-F121:

IY-F121
-------

重大度: エラー

メッセージ::

    error[IY-F121]: <SU2> was killed by signal <n>; end of <log>:
    <last lines>

意味
    SU2 がシグナルで終了した。

主な原因
    外部から止められた、または異常終了した。

対処
    ログの末尾を読む。

関連
    :ref:`sec-cfd-run`

.. _IY-F122:

IY-F122
-------

重大度: エラー

メッセージ::

    error[IY-F122]: <SU2> exited with <status>; end of <log>:
    <last lines>

意味
    SU2 が異常終了コードで終了した。

主な原因
    設定の誤り、メッシュの不具合、発散。

対処
    ログの末尾を読む。

関連
    :ref:`sec-cfd-run`

.. _IY-F123:

IY-F123
-------

重大度: エラー

メッセージ::

    error[IY-F123]: the CFD mode needs a Unix-like system (Linux, macOS or WSL)

意味
    CFD は Unix 系 OS でのみ動く。

主な原因
    Windows のネイティブ環境で実行した。

対処
    Linux、macOS、WSL で実行する。

関連
    :ref:`sec-cfd-native`

.. _IY-F501:

IY-F501
-------

重大度: ワーニング

メッセージ::

    warning[IY-F501]: SU2 <a>.<b>.<c> found; the documentation and validation are pinned to SU2 8.3.0, results may differ

意味
    SU2 のバージョンが 8.3.x ではない。

主な原因
    別バージョンの SU2 を使っている。

対処
    8.3.0 を導入するか、結果の差に注意する。

関連
    :ref:`sec-cfd-native`

.. _IY-F502:

IY-F502
-------

重大度: ワーニング

メッセージ::

    warning[IY-F502]: cannot read the SU2 version from "<banner>"; the documentation and validation are pinned to SU2 8.3.0

意味
    SU2 の出力からバージョンを読み取れない。

主な原因
    想定外の出力形式である。

対処
    ``SU2_CFD --help`` を確認する。

関連
    :ref:`sec-cfd-native`

.. _IY-F503:

IY-F503
-------

重大度: ワーニング

メッセージ::

    warning[IY-F503]: <SU2_CFD> was compiled for the CPU "<built>" but this machine has "<current>"; it may stop with an illegal instruction. Run `install.sh --cfd --force` here to rebuild it, or `install.sh --cfd --su2-binary` for the portable conda-forge SU2

意味
    ソースからビルドした SU2 が、現在とは別の CPU 向けである。

主な原因
    ビルドした計算機と違う計算機で実行している。

対処
    ``install.sh --cfd --force`` で再ビルドするか ``--su2-binary`` を使う。

関連
    :ref:`sec-cfd-native`

.. _IY-F504:

IY-F504
-------

重大度: ワーニング

メッセージ::

    warning[IY-F504]: roll damping at M=<M>: using the Barrowman Clp (<value>): <reason>

意味
    そのマッハ数のロール減衰を CFD から得られず、Barrowman の値を使う。

主な原因
    ロール用ケースが失敗した。

対処
    理由を確認する。

関連
    :ref:`sec-cfd-roll-cases`、:ref:`sec-cfd-roll-post`

.. _IY-F505:

IY-F505
-------

重大度: ワーニング

メッセージ::

    warning[IY-F505]: RANS: some cases have no per-marker CFx columns; the table takes the base from CFD for all of them

意味
    一部のケースに境界ごとの CFx 列がないため、全ケースで底面抗力を CFD から取る。

主な原因
    ケースの SU2 出力列が揃っていない。

対処
    対応は不要である。

関連
    :ref:`sec-cfd-rans-use`

.. _IY-F506:

IY-F506
-------

重大度: ワーニング

メッセージ::

    warning[IY-F506]: RANS: the history has no per-marker CFx(wall) column; the axial force is the monitored total, which includes the base cap

意味
    壁面の CFx 列がないため、軸力は底面を含む合計である。

主な原因
    SU2 の出力列がない。

対処
    対応は不要である。

関連
    :ref:`sec-cfd-rans-use`

.. _IY-F507:

IY-F507
-------

重大度: ワーニング

メッセージ::

    warning[IY-F507]: the table is held at its M = <M> values above | below M = <M> (no Mach extrapolation of the CFD data)

意味
    CFD の最大（最小）マッハ数の外側では、端の値を保持する。

主な原因
    計算したマッハ数の範囲がテーブルの範囲より狭い。

対処
    範囲を広げて計算する。

関連
    :ref:`sec-cfd-table-build`

.. _IY-F508:

IY-F508
-------

重大度: ワーニング

メッセージ::

    warning[IY-F508]: <n> RANS case(s) accepted with a stalled residual, not converged (coefficients averaged over two windows; a stalled residual needs only <x> orders below the reference, convergence <y>): <cases>; check them in cfd_cases.csv (rans_verdict, residual_drop)

意味
    残差が止まったまま受け入れた RANS ケースがある。

主な原因
    残差が基準まで下がりきらなかった。

対処
    cfd_cases.csv で該当ケースを確認する。

関連
    :ref:`sec-cfd-int-rans-accept`

.. _IY-F509:

IY-F509
-------

重大度: ワーニング

メッセージ::

    warning[IY-F509]: zero-angle asymmetry at M=<M>: <force> at alpha = beta = 0 (<x> % of CN at <a> deg = <cn>)<hint>; <fate>

意味
    迎角 0 度で、対称な機体なら 0 になる横力が出ている。

主な原因
    フィンの分解能不足、底面の非対称な後流、メッシュの非対称。

対処
    メッシュを細かくする、alpha_mode = "mirror" か "offset" にする。

関連
    :ref:`sec-cfd-alpha-mode`

.. _IY-F510:

IY-F510
-------

重大度: ワーニング

メッセージ::

    warning[IY-F510]: base_treatment = flat: the flat base is kept; the inviscid flow behind it is ill-posed, so the Euler solution is likely not to converge (cases that do not converge fail); the base is not integrated, the base drag comes from the empirical base term

意味
    Euler で平らな底面をそのまま使う。

主な原因
    base_treatment = "flat" の指定。

対処
    収束しないときは base_treatment = "fairing" を試す。

関連
    :ref:`sec-cfd-stl-base`

.. _IY-F511:

IY-F511
-------

重大度: ワーニング

メッセージ::

    warning[IY-F511]: base_treatment = fairing: the <input STL|CAD shape> is MODIFIED, a closed cone of <n> base radii (cut at 10 % of the base radius) is united with the body behind the base; the base is not integrated, the base drag comes from the empirical base term

意味
    底面の後ろに円錐を融合するため、形状を変更する。

主な原因
    base_treatment = "fairing" の指定。

対処
    対応は不要である。

関連
    :ref:`sec-cfd-stl-base`、:ref:`sec-cfd-fairing`

.. _IY-F512:

IY-F512
-------

重大度: ワーニング

メッセージ::

    warning[IY-F512]: the CAD file holds <n> disjoint solids; all of them are walls of the CFD mesh

意味
    CAD ファイルに離れた複数のソリッドがあり、すべてが壁になる。

主な原因
    別部品を含んだまま書き出した。

対処
    不要な部品を除いて書き出す。

関連
    :ref:`sec-cfd-step`

.. _IY-F513:

IY-F513
-------

重大度: ワーニング

メッセージ::

    warning[IY-F513]: geometry error <e> m exceeds half the wall size (<w> m)

意味
    壁面の形状誤差が wall_size の半分を超えている（失敗の閾値は max_geometry_error）。

主な原因
    wall_size に対して形状が細かい。

対処
    wall_size を小さくするか stl_remesh を見直す。

関連
    :ref:`sec-cfd-fidelity`

.. _IY-F520:

IY-F520
-------

重大度: ワーニング

メッセージ::

    warning[IY-F520]: unconverged: <reason>; coefficients averaged over the last <n> iterations

意味
    残差が止まったままケースを受け入れた。

主な原因
    残差が基準まで下がらなかった。

対処
    cfd_cases.csv を確認する。

関連
    :ref:`sec-cfd-int-rans-accept`

.. _IY-F521:

IY-F521
-------

重大度: ワーニング

メッセージ::

    warning[IY-F521]: limit cycle: coefficients averaged over the last <n> iterations

意味
    係数が周期的に振動するため、窓で平均した。

主な原因
    底面後流などの非定常性。

対処
    対応は不要である。

関連
    :ref:`sec-cfd-int-rans-accept`

.. _IY-F522:

IY-F522
-------

重大度: ワーニング

メッセージ::

    warning[IY-F522]: wall y+ P95 = <y> exceeds <max> (target yplus = <t>): the first cell is too thick for the resolved wall

意味
    壁面 y+ の 95 パーセンタイルが上限を超えている。

主な原因
    第 1 層が厚すぎる。

対処
    yplus を下げる。

関連
    :ref:`sec-cfd-prism`

.. _IY-F523:

IY-F523
-------

重大度: ワーニング

メッセージ::

    warning[IY-F523]: wall y+ not checked: no Y_Plus in surface_flow.vtk

意味
    surface_flow.vtk に Y_Plus がなく y+ を確認していない。

主な原因
    SU2 の出力設定。

対処
    対応は不要である。

関連
    :ref:`sec-cfd-prism`

.. _IY-F524:

IY-F524
-------

重大度: ワーニング

メッセージ::

    warning[IY-F524]: the coefficients still trend or oscillate at the end (<n> iterations): their range over the last stable_window iterations or their change extrapolated from the residual decay exceeds <x> % of the force scale (centre of pressure: <y> body lengths); coefficients averaged over the last <m> iterations

意味
    Euler のケースを受け入れたが、係数がまだ動いている。

主な原因
    反復が足りない。

対処
    iterations を増やす。

関連
    :ref:`sec-cfd-int-convergence`

.. _IY-F525:

IY-F525
-------

重大度: ワーニング

メッセージ::

    warning[IY-F525]: the warm start failed (<reason>); solved again from the freestream

意味
    前のケースからの暖機スタートが失敗し、一様流から計算し直した。

主な原因
    暖機元の解が合わなかった。

対処
    対応は不要である。

関連
    :ref:`sec-cfd-int-rans-stages`

.. _IY-F526:

IY-F526
-------

重大度: ワーニング

メッセージ::

    warning[IY-F526]: the first-order startup diverged (<reason>); repeated once with the adaptive CFL capped at <a> instead of <b> (the SA equation stays at <c> of the flow CFL); the second-order stage keeps these settings

意味
    1 次精度の立ち上げが発散したので、CFL を抑えてやり直した。

主な原因
    メッシュ品質や条件が厳しい。

対処
    対応は不要である。

関連
    :ref:`sec-cfd-int-rans-stages`

.. _IY-F527:

IY-F527
-------

重大度: ワーニング

メッセージ::

    warning[IY-F527]: the second-order stage diverged (<reason>); repeated once from the first-order solution with the adaptive CFL capped at <a> instead of <b> and the SA equation at <c> of the flow CFL instead of 1

意味
    2 次精度の段が発散したので、CFL を抑えてやり直した。

主な原因
    メッシュ品質や条件が厳しい。

対処
    対応は不要である。

関連
    :ref:`sec-cfd-int-rans-stages`
