.. _errors-geometry:

形状のエラーとワーニング
========================

形状の読み込みと抽出（``geom`` と、STL を読むその他のコマンド）が出力するエラーとワーニングの一覧である。エラーは ``error[IY-Gnnn]: …``、ワーニングは ``warning[IY-Gnnn]: …`` の形で表示する。
山括弧 ``<…>`` は実行時の値に置き換わる箇所である。ワーニングは ``geometry.json`` の ``warnings`` にも書く。そこでは表示用の ``warning[…]:`` の形にせず、``[IY-Gnnn] …`` の文字列のままにする。

.. _IY-G001:

IY-G001
-------

重大度: エラー

メッセージ::

    error[IY-G001]: cannot read STL <path>

意味
    STL ファイルを開けない、または読めない。

主な原因
    パスの誤り、ファイルがない、読み取り権限がない。

対処
    ``rocket.stl`` のパスと権限を確認する。

関連
    :ref:`page-theory-geometry`

.. _IY-G002:

IY-G002
-------

重大度: エラー

メッセージ::

    error[IY-G002]: binary STL declares <n> triangles (needs <bytes> bytes) but the file has <size> bytes: the file is truncated or corrupt

意味
    バイナリ STL の見出しが宣言する三角形数に対して、ファイルが短い。

主な原因
    転送やダウンロードの途中で切れた、書き出しが中断した、ファイルが壊れている。

対処
    CAD から STL を書き出し直す。ファイルのサイズが転送前後で一致するか確かめる。

関連
    :ref:`page-theory-geometry`

.. _IY-G003:

IY-G003
-------

重大度: エラー

メッセージ::

    error[IY-G003]: unrecognised STL format

意味
    ASCII の STL にもバイナリの STL にも見えない。

主な原因
    STL ではないファイルを指定した、84 バイトに満たない、UTF-8 でない ASCII 風の文字列である。

対処
    ``rocket.stl`` が STL ファイルを指しているか確認する。他の形式（STEP、OBJ など）は STL に変換する。

関連
    :ref:`page-theory-geometry`

.. _IY-G004:

IY-G004
-------

重大度: エラー

メッセージ::

    error[IY-G004]: STL contains no triangles

意味
    STL に三角形が 1 枚もない。

主な原因
    空の STL、``facet`` を含まない ASCII STL。

対処
    CAD で実体を選んで書き出し直す。

関連
    :ref:`page-theory-geometry`

.. _IY-G005:

IY-G005
-------

重大度: エラー

メッセージ::

    error[IY-G005]: triangle <i> has a non-finite (NaN or infinite) vertex coordinate

意味
    ``<i>`` 番目の三角形の頂点座標が NaN または無限大である。

主な原因
    書き出し元の不具合、ファイルの破損。

対処
    CAD で書き出し直す。壊れたメッシュを修復してから書き出す。

関連
    :ref:`page-theory-geometry`

.. _IY-G006:

IY-G006
-------

重大度: エラー

メッセージ::

    error[IY-G006]: malformed vertex line: <line>

意味
    ASCII STL の ``vertex`` 行に座標が 3 つない。

主な原因
    手で編集した、行が欠けた、ファイルが壊れている。

対処
    該当の行を直すか、STL を書き出し直す。

関連
    :ref:`page-theory-geometry`

.. _IY-G007:

IY-G007
-------

重大度: エラー

メッセージ::

    error[IY-G007]: ASCII STL is not valid UTF-8

意味
    ASCII STL の文字列を UTF-8 として読めない。

主な原因
    文字コードが異なる、ファイルが壊れている。

対処
    STL を書き出し直す（バイナリ形式を選んでもよい）。

関連
    :ref:`page-theory-geometry`

.. _IY-G008:

IY-G008
-------

重大度: エラー

メッセージ::

    error[IY-G008]: invalid number in vertex line: <line>

意味
    ASCII STL の ``vertex`` 行に、数値として読めない値がある。

主な原因
    座標が数値でない、小数点の表記が異なる、ファイルが壊れている。

対処
    該当の行を直すか、STL を書き出し直す。

関連
    :ref:`page-theory-geometry`

.. _IY-G009:

IY-G009
-------

重大度: エラー

メッセージ::

    error[IY-G009]: cannot write STL <path>

意味
    STL ファイルを書けない。

主な原因
    出力先のディレクトリがない、書き込み権限がない、ディスクの空きがない。

対処
    出力先のパスと権限、空き容量を確認する。

関連
    :ref:`page-theory-geometry`

.. _IY-G010:

IY-G010
-------

重大度: エラー

メッセージ::

    error[IY-G010]: the STL surface is not closed: <open> open edges of <edges> (<percent> %): the geometry extraction needs a closed (watertight) surface; repair the mesh (close the holes) or export the rocket as a solid

意味
    開いた辺（1 枚の三角形にしか属さない辺）が全体の辺の 1 % を超えており、形状抽出に使えない。

主な原因
    端面やフィンの面が抜けている、機体を面（サーフェス）のまま書き出した、メッシュに多数の穴がある。

対処
    CAD でメッシュの穴を塞ぐか、機体を ``ソリッド`` として書き出し直す。

関連
    :ref:`sec-geometry-warnings`

.. _IY-G011:

IY-G011
-------

重大度: エラー

メッセージ::

    error[IY-G011]: the STL surface is not closed: a hole bounded by <n> open edges is <extent> m across (<percent> % of the body diameter <diameter> m): the geometry extraction needs a closed (watertight) surface; repair the mesh (close the holes) or export the rocket as a solid

意味
    開いた辺の割合は 1 % 以下だが、最も広い穴の幅が胴体直径の 0.1 倍を超えている。底面の蓋、フィンの面、胴体の一部が抜けていると、細かいメッシュでは割合だけでは検出できない。

主な原因
    底面の蓋やフィンの面が抜けている、胴体の一部が欠けている。

対処
    IY-G010 と同じ。メッシュの穴を塞ぐか、ソリッドとして書き出し直す。

関連
    :ref:`sec-geometry-warnings`

.. _IY-G012:

IY-G012
-------

重大度: エラー

メッセージ::

    error[IY-G012]: n_slices must be at least 20

意味
    断面の数 ``aero.n_slices`` が 20 未満である。

主な原因
    設定値が小さすぎる。

対処
    ``aero.n_slices`` を 20 以上にする（既定値を使うなら指定しない）。

関連
    :ref:`page-theory-geometry`

.. _IY-G013:

IY-G013
-------

重大度: エラー

メッセージ::

    error[IY-G013]: mesh has zero surface area

意味
    メッシュの表面積が 0 で、形状を抽出できない。

主な原因
    すべての三角形が退化している、尺度が 0 である（``rocket.stl_scale``）。

対処
    STL の内容と ``rocket.stl_scale`` を確認する。

関連
    :ref:`page-theory-geometry`

.. _IY-G014:

IY-G014
-------

重大度: エラー

メッセージ::

    error[IY-G014]: degenerate mesh

意味
    メッシュの機軸方向の長さが 0 で、形状を抽出できない。

主な原因
    STL が 1 点や平面に潰れている。

対処
    STL の内容を確認する。

関連
    :ref:`page-theory-geometry`

.. _IY-G015:

IY-G015
-------

重大度: エラー

メッセージ::

    error[IY-G015]: could not determine body radius

意味
    胴体の半径を決められない（どの断面にも胴体の幅がない）。

主な原因
    STL が機体の形をしていない、尺度が極端に小さい。

対処
    STL が機体全体を含むか、``rocket.stl_scale`` が正しいかを確認する。

関連
    :ref:`page-theory-geometry`

.. _IY-G501:

IY-G501
-------

重大度: ワーニング

メッセージ::

    warning[IY-G501]: the STL surface has <n> open and <m> non-manifold (shared by more than two triangles) edges of <edges>: small mesh defects, check the extracted geometry

意味
    STL の表面に少数の開いた辺、または 3 枚以上の三角形が共有する辺（非多様体の辺）がある。開いた辺は 1 % 以下で、抽出は続行した。メッセージは該当する辺の種類だけを書く（開いた辺だけ、非多様体の辺だけ、または両方）。

主な原因
    小さな穴、重なった三角形、結合し損ねた部品。

対処
    抽出した形状（``geometry.json``、``geom`` の表示）が正しいか確認する。気になるときはメッシュを修復する。

関連
    :ref:`sec-geometry-warnings`

.. _IY-G502:

IY-G502
-------

重大度: ワーニング

メッセージ::

    warning[IY-G502]: a short feature with radius <max> m exceeds the typical body radius <p90> m and was ignored

意味
    短い突起（リングやカラー）が胴体の代表半径を大きく超えるため、基準半径の決定から除いた。

主な原因
    胴体に短い環状の張り出しがある。

対処
    基準半径が意図どおりか確認する。突起が胴体の一部なら、形状を修正する。

関連
    :ref:`sec-geometry-warnings`

.. _IY-G503:

IY-G503
-------

重大度: ワーニング

メッセージ::

    warning[IY-G503]: the nose/tail shape and the fin position suggest opposite orientations; the fin position was used (set nose_direction to override)

意味
    端の半径から見た向きと、フィンの位置から見た向きが食い違う。フィンの位置を採用した。

主な原因
    ボートテールや突き出したノズルがある、機体が逆向きである。

対処
    抽出したノーズと底面が正しいか確認し、必要なら ``rocket.nose_direction`` を指定する。

関連
    :ref:`sec-geometry-orientation`

.. _IY-G504:

IY-G504
-------

重大度: ワーニング

メッセージ::

    warning[IY-G504]: orientation decided from the end radii only (no clear fin set); check the extracted nose/tail or set nose_direction

意味
    フィンを判別できず、端の半径だけでノーズの向きを決めた。

主な原因
    フィンのない機体、フィンが小さい、フィンの位置が中央に近い。

対処
    抽出したノーズと底面を確認するか、``rocket.nose_direction`` を指定する。

関連
    :ref:`sec-geometry-orientation`

.. _IY-G505:

IY-G505
-------

重大度: ワーニング

メッセージ::

    warning[IY-G505]: length/diameter ratio <ratio> is unusual for a rocket (length <length> m, diameter <d> m); check the STL scale and units

意味
    全長と基準直径の比が 3 未満または 40 を超える。

主な原因
    STL の尺度や単位が違う（mm と m の取り違えなど）、機体が極端に細長い、または太い。

対処
    ``rocket.stl_scale`` を確認する。

関連
    :ref:`sec-geometry-warnings`

.. _IY-G506:

IY-G506
-------

重大度: ワーニング

メッセージ::

    warning[IY-G506]: detected nose length <length> m (<calibers> calibers) is implausible

意味
    検出したノーズ長が基準直径の 0.4 倍未満または 8 倍を超える。

主な原因
    ノーズの形が特殊である、向きの誤判定、肩の位置の誤検出。

対処
    抽出したノーズを確認し、必要なら ``rocket.nose_direction`` を指定する。

関連
    :ref:`sec-geometry-warnings`

.. _IY-G507:

IY-G507
-------

重大度: ワーニング

メッセージ::

    warning[IY-G507]: the aft end of the mesh is wider than the body (radius <base> m vs <r_max> m): the fins probably extend behind the body base; Barrowman's fin-body interference assumes the body continues behind the fin root chord and base drag uses this radius (check the fins, or set rocket.fin_override)

意味
    メッシュの後端が胴体より広い。フィンが胴体の底面より後ろに出ている可能性が高い。

主な原因
    フィンの後縁が胴体の後端より後ろにある。

対処
    フィンの抽出結果を確認するか、``rocket.fin_override`` でフィンを指定する。

関連
    :ref:`sec-geometry-warnings`

.. _IY-G508:

IY-G508
-------

重大度: ワーニング

メッセージ::

    warning[IY-G508]: the base radius is only <percent> % of the body radius (protruding nozzle or strong boattail); base drag uses this radius

意味
    底面半径が胴体半径の 60 % 未満である。底面抗力はこの半径で計算する。

主な原因
    突き出したノズル、強いボートテール。

対処
    形状が意図どおりか確認する。

関連
    :ref:`sec-geometry-warnings`

.. _IY-G509:

IY-G509
-------

重大度: ワーニング

メッセージ::

    warning[IY-G509]: the fin root leading edge is at <percent> % of the length, in the forward half; the body may be reversed (nose_direction)

意味
    フィンの根元前縁が全長の前半にある。

主な原因
    機体の向きの誤判定。

対処
    ``rocket.nose_direction`` を確認する。

関連
    :ref:`sec-geometry-orientation`

.. _IY-G510:

IY-G510
-------

重大度: ワーニング

メッセージ::

    warning[IY-G510]: the fins are centred at <percent> % of the length, unusually far forward

意味
    フィン根元の中央が全長の 60 % より前にある。

主な原因
    フィンが前方にある設計、向きの誤判定。

対処
    フィンの位置と ``rocket.nose_direction`` を確認する。

関連
    :ref:`sec-geometry-warnings`

.. _IY-G511:

IY-G511
-------

重大度: ワーニング

メッセージ::

    warning[IY-G511]: the fin span <span> m exceeds two body diameters (<d> m)

意味
    フィンのスパンが基準直径の 2 倍を超える。

主な原因
    スパンの大きい設計、尺度の誤り、他の部品をフィンと誤検出した。

対処
    フィンの抽出結果を確認する。

関連
    :ref:`sec-geometry-warnings`

.. _IY-G512:

IY-G512
-------

重大度: ワーニング

メッセージ::

    warning[IY-G512]: the fin trailing edge at <te> m lies aft of the body end at <length> m: Barrowman's fin-body interference assumes that the body continues behind the fin root chord

意味
    フィンの後縁（根元または先端）が胴体の後端より後ろにある。

主な原因
    フィンが胴体の後端から張り出している設計、``rocket.fin_override`` の寸法が合っていない。

対処
    フィンの位置を確認する。

関連
    :ref:`sec-geometry-warnings`

