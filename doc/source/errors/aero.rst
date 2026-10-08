.. _errors-aero:

空力のエラーとワーニング
========================

空力係数表の読み込み（``aero.method = "table"`` を含む）、大気モデル、パネル法の設定とメモリの検査が出力するエラーとワーニングの一覧である。エラーは ``error[IY-Annn]: …``、ワーニングは ``warning[IY-Annn]: …`` の形で表示する。ワーニングを ``summary.json`` などの ``warnings`` 配列に書くときは、``[IY-Annn] …`` の文字列のままにする。
山括弧 ``<…>`` は実行時の値に置き換わる箇所である。

.. _IY-A001:

IY-A001
-------

重大度: エラー

メッセージ::

    error[IY-A001]: atmosphere.temperature_offset must be finite, got <value>

意味
    ``atmosphere.temperature_offset`` が NaN または無限大である。

主な原因
    設定値の誤り。

対処
    有限の値 [K] を指定する。

関連
    :ref:`page-theory-atmosphere-gravity`

.. _IY-A002:

IY-A002
-------

重大度: エラー

メッセージ::

    error[IY-A002]: atmosphere.temperature_offset = <value> K gives a non-positive absolute temperature (the standard profile reaches <t_min> K at the top of the supported altitude range); it must be greater than <limit> K

意味
    温度のオフセットが大きな負の値で、標準大気のどこかで絶対温度が 0 以下になる。

主な原因
    オフセットの符号や単位の誤り。

対処
    オフセットを表示された下限より大きくする。

関連
    :ref:`page-theory-atmosphere-gravity`

.. _IY-A003:

IY-A003
-------

重大度: エラー

メッセージ::

    error[IY-A003]: atmosphere.<name> must be finite and positive for the constant atmosphere, got <value>

意味
    一定大気（``atmosphere.model = "constant"``）の ``density``、``sound_speed``、``viscosity`` のいずれかが有限の正の値でない。

主な原因
    設定値の誤り、0 や負の値、NaN。

対処
    表示された項目に正の値を指定する。

関連
    :ref:`page-theory-atmosphere-gravity`

.. _IY-A010:

IY-A010
-------

重大度: エラー

メッセージ::

    error[IY-A010]: aero.panel.body_axial must be at least 6, found <value>

意味
    ``aero.panel.body_axial`` が許される範囲にない（6 未満）。

主な原因
    設定値の誤り。

対処
    6 以上の整数にする。

関連
    :ref:`page-theory-panel`

.. _IY-A011:

IY-A011
-------

重大度: エラー

メッセージ::

    error[IY-A011]: aero.panel.body_circ must be at least 8, found <value>

意味
    ``aero.panel.body_circ`` が許される範囲にない（8 未満）。

主な原因
    設定値の誤り。

対処
    8 以上の整数にする。

関連
    :ref:`page-theory-panel`

.. _IY-A012:

IY-A012
-------

重大度: エラー

メッセージ::

    error[IY-A012]: aero.panel.fin_chord must be between 2 and aero.panel.body_axial - 4 = <max> (the chord panels are part of the body stations), found <value>

意味
    ``aero.panel.fin_chord`` が許される範囲にない（2 から ``body_axial - 4`` の範囲外）。

主な原因
    設定値の誤り。

対処
    2 以上 ``body_axial - 4`` 以下にする。コード方向のパネルは胴体の軸方向の分割に含まれる。

関連
    :ref:`page-theory-panel`

.. _IY-A013:

IY-A013
-------

重大度: エラー

メッセージ::

    error[IY-A013]: aero.panel.fin_span must be at least 2, found <value>

意味
    ``aero.panel.fin_span`` が許される範囲にない（2 未満）。

主な原因
    設定値の誤り。

対処
    2 以上の整数にする。

関連
    :ref:`page-theory-panel`

.. _IY-A014:

IY-A014
-------

重大度: エラー

メッセージ::

    error[IY-A014]: aero.panel.wake_length must be a finite number > 0 (body lengths), found <value>

意味
    ``aero.panel.wake_length`` が許される範囲にない（有限の正の数でない）。

主な原因
    設定値の誤り。

対処
    全長を単位とする正の数にする。

関連
    :ref:`page-theory-panel`

.. _IY-A015:

IY-A015
-------

重大度: エラー

メッセージ::

    error[IY-A015]: aero.panel.tail_radii must be a finite number >= 0 (base radii; 0 = open base), found <value>

意味
    ``aero.panel.tail_radii`` が許される範囲にない（有限の 0 以上の数でない）。

主な原因
    設定値の誤り。

対処
    0 以上の数にする（0 は底面を開いたままにする）。

関連
    :ref:`page-theory-panel`

.. _IY-A016:

IY-A016
-------

重大度: エラー

メッセージ::

    error[IY-A016]: aero.panel.subsonic_machs must be a non-empty list of finite Mach numbers in [0, 1), found <list>

意味
    ``aero.panel.subsonic_machs`` が許される範囲にない（空である、または 0 以上 1 未満の有限の数でない値を含む）。

主な原因
    設定値の誤り。

対処
    0 以上 1 未満の Mach 数を 1 つ以上並べる。

関連
    :ref:`page-theory-panel`

.. _IY-A017:

IY-A017
-------

重大度: エラー

メッセージ::

    error[IY-A017]: aero.panel.transonic = [<lo>, <hi>] must be finite and increasing, [lo, hi] with 0 < lo and hi > 1 (hi is the first fully supersonic Mach number)

意味
    ``aero.panel.transonic`` が許される範囲にない（``[lo, hi]`` が有限で増加していない、``lo`` が 0 以下、または ``hi`` が 1 以下）。

主な原因
    設定値の誤り。

対処
    ``0 < lo < hi`` かつ ``hi > 1`` にする。``hi`` は完全に超音速とみなす最初の Mach 数である。

関連
    :ref:`page-theory-panel`

.. _IY-A018:

IY-A018
-------

重大度: エラー

メッセージ::

    error[IY-A018]: the panel method needs about <need> GB for <n> panels (dense matrix), more than the <available> GB of memory available to this process (<source>); use coarser panel settings (body_axial, body_circ, fin_chord, fin_span)

意味
    パネル法の密行列がこのプロセスで使えるメモリに入らない。括弧内は使えるメモリの値を取った出所である。

主な原因
    パネル数が多すぎる、他のプログラムがメモリを使っている、cgroup やジョブの制限が小さい。

対処
    ``body_axial``、``body_circ``、``fin_chord``、``fin_span`` を小さくするか、メモリを空ける。どの出所も読めない環境ではこの検査をしない。

関連
    :ref:`page-theory-panel`

.. _IY-A019:

IY-A019
-------

重大度: エラー

メッセージ::

    error[IY-A019]: the panel method needs about <need> GB for <n> panels (dense matrix), above resources.memory_gb = <limit>; use coarser panel settings (body_axial, body_circ, fin_chord, fin_span) or raise the budget

意味
    パネル法の密行列が ``resources.memory_gb`` の予算を超える。

主な原因
    パネル数が多すぎる、予算が小さい。

対処
    パネルの設定を粗くするか、``resources.memory_gb`` を上げる。

関連
    :ref:`page-theory-panel`

.. _IY-A020:

IY-A020
-------

重大度: エラー

メッセージ::

    error[IY-A020]: could not allocate the dense panel matrix: about <need> GB for <n> panels; use coarser panel settings (body_axial, body_circ, fin_chord, fin_span) or free memory

意味
    密行列のメモリを確保できなかった。事前の見積もり（IY-A018、IY-A019）を通っても、実際の確保に失敗することがある。

主な原因
    空きメモリの不足、アドレス空間の制限（``ulimit -v`` など）。

対処
    パネルの設定を粗くするか、メモリを空ける。

関連
    :ref:`page-theory-panel`

.. _IY-A021:

IY-A021
-------

重大度: エラー

メッセージ::

    error[IY-A021]: panel mesh too coarse (body_axial >= 6, body_circ >= 8 required)

意味
    パネルメッシュを作る時点で、胴体の分割数が最小値に満たない。

主な原因
    ``body_axial`` または ``body_circ`` が小さい（IY-A010、IY-A011 と同じ条件）。

対処
    ``body_axial`` を 6 以上、``body_circ`` を 8 以上にする。

関連
    :ref:`page-theory-panel`

.. _IY-A022:

IY-A022
-------

重大度: エラー

メッセージ::

    error[IY-A022]: unsupported fin section

意味
    パネル法が扱えないフィン断面が指定された。

主な原因
    両凸（biconvex）以外の断面を指定した。

対処
    両凸の断面にする。

関連
    :ref:`page-theory-panel`

.. _IY-A023:

IY-A023
-------

重大度: エラー

メッセージ::

    error[IY-A023]: fin root lies outside the body

意味
    フィンの根元の弦が胴体の範囲に収まらない（前縁が後縁以上の位置にある）。

主な原因
    抽出したフィンの位置が胴体の後端より後ろにある、``rocket.fin_override`` の寸法の誤り。

対処
    フィンの抽出結果と ``rocket.fin_override`` を確認する。

関連
    :ref:`page-theory-panel`

.. _IY-A030:

IY-A030
-------

重大度: エラー

メッセージ::

    error[IY-A030]: <axis> axis is empty

意味
    係数表のメタデータの軸（``machs``、``alphas_deg``、``betas_deg``）が空である。

主な原因
    メタデータの書き間違い。

対処
    軸に値を 1 つ以上書く。

関連
    :ref:`sec-file-sidecar`

.. _IY-A031:

IY-A031
-------

重大度: エラー

メッセージ::

    error[IY-A031]: <axis>[<i>] is not finite: <value>

意味
    メタデータの軸に NaN または無限大がある。

主な原因
    メタデータの書き間違い。

対処
    有限の値にする。

関連
    :ref:`sec-file-sidecar`

.. _IY-A032:

IY-A032
-------

重大度: エラー

メッセージ::

    error[IY-A032]: <axis> must be strictly increasing, but <axis>[<i+1>] = <value> follows <axis>[<i>] = <value>

意味
    メタデータの軸が狭義の単調増加でない。

主な原因
    並びの誤り、同じ値の重複。

対処
    軸を昇順にし、重複を除く。

関連
    :ref:`sec-file-sidecar`

.. _IY-A033:

IY-A033
-------

重大度: エラー

メッセージ::

    error[IY-A033]: <name> must be finite and positive, got <value>

意味
    メタデータの ``ref_area``、``ref_diameter``、``length`` のいずれかが有限の正の値でない。

主な原因
    メタデータの書き間違い、単位の誤り。

対処
    表示された項目に正の値を書く。

関連
    :ref:`sec-file-sidecar`

.. _IY-A034:

IY-A034
-------

重大度: エラー

メッセージ::

    error[IY-A034]: fin_cant_ref_deg must be finite, got <value>

意味
    メタデータの ``fin_cant_ref_deg`` が有限でない。

主な原因
    メタデータの書き間違い。

対処
    有限の角度 [deg] を書く。

関連
    :ref:`sec-file-sidecar`

.. _IY-A035:

IY-A035
-------

重大度: エラー

メッセージ::

    error[IY-A035]: fin_roll_deg must be finite, got <value>

意味
    メタデータの ``fin_roll_deg`` が有限でない。

主な原因
    メタデータの書き間違い。

対処
    有限の角度 [deg] を書く。

関連
    :ref:`sec-file-sidecar`

.. _IY-A036:

IY-A036
-------

重大度: エラー

メッセージ::

    error[IY-A036]: moment_ref "<value>" is not supported: tabulated moments must be about the "nose_tip" (the simulator transfers them to the CG)

意味
    メタデータの ``moment_ref`` が ``nose_tip`` でない。係数表のモーメントは機首先端まわりでなければならず、シミュレータが重心まわりに移す。

主な原因
    他の基準点まわりで作った表を指定した。

対処
    表を機首先端まわりのモーメントで作り直し、``moment_ref`` を ``"nose_tip"`` にする。

関連
    :ref:`sec-file-sidecar`

.. _IY-A037:

IY-A037
-------

重大度: エラー

メッセージ::

    error[IY-A037]: alphas_deg of an axisymmetric table must start at 0 deg (the total angle of attack), found <value>: the small-angle coefficients would be extrapolated

意味
    軸対称の表の迎角の軸が 0 度から始まっていない。微小角の係数が外挿になってしまう。

主な原因
    軸が負の角度や 0 度以外から始まっている。

対処
    迎角の軸を 0 度から始める。

関連
    :ref:`sec-file-axisym`

.. _IY-A038:

IY-A038
-------

重大度: エラー

メッセージ::

    error[IY-A038]: <axis> of a general table must include 0 deg between its first and last value, found <lo> to <hi>: the small-angle coefficients would be extrapolated

意味
    一般の表の迎角または横滑り角の軸が 0 度をまたいでいない。

主な原因
    軸が片側の角度だけを含む。

対処
    両方の軸の最小値を 0 以下、最大値を 0 以上にする。

関連
    :ref:`sec-file-general`

.. _IY-A040:

IY-A040
-------

重大度: エラー

メッセージ::

    error[IY-A040]: cannot write <path>

意味
    係数表の CSV またはメタデータの JSON を書けない。

主な原因
    出力先がない、権限がない、ディスクの空きがない。

対処
    出力先のパス、権限、空き容量を確認する。

関連
    :ref:`page-reference-aero-table-format`

.. _IY-A041:

IY-A041
-------

重大度: エラー

メッセージ::

    error[IY-A041]: cannot read aero table metadata <path>

意味
    係数表のメタデータ（CSV と同じ名前の ``.json``）を読めない。

主な原因
    ファイルがない、権限がない。

対処
    CSV の横に同名の ``.json`` を置く。

関連
    :ref:`sec-file-sidecar`

.. _IY-A042:

IY-A042
-------

重大度: エラー

メッセージ::

    error[IY-A042]: bad aero table metadata <path>

意味
    メタデータが JSON として、または項目の型として不正である。

主な原因
    JSON の文法の誤り、必須項目の欠落、項目の型の誤り。

対処
    メッセージに続く詳細（行と列）を見て直す。

関連
    :ref:`sec-file-sidecar`

.. _IY-A043:

IY-A043
-------

重大度: エラー

メッセージ::

    error[IY-A043]: cannot read aero table <path>

意味
    係数表の CSV を読めない。

主な原因
    ファイルがない、権限がない、UTF-8 でない。

対処
    パスと権限を確認する。

関連
    :ref:`page-reference-aero-table-format`

.. _IY-A044:

IY-A044
-------

重大度: エラー

メッセージ::

    error[IY-A044]: aero table <path>: unexpected CSV header `<found>`; expected `<expected>`

意味
    CSV の見出しの列の名前、順序、列数が期待と一致しない（大文字小文字と前後の空白は区別しない）。

主な原因
    列を入れ替えた、列を足した、列を削った、軸対称の表と一般の表を取り違えた。

対処
    表示された見出しに合わせる。

関連
    :ref:`page-reference-aero-table-format`

.. _IY-A045:

IY-A045
-------

重大度: エラー

メッセージ::

    error[IY-A045]: bad row <line> in <path>: <row text>

意味
    行に数値として読めない値がある。

主な原因
    空欄、文字、小数点の表記の誤り。

対処
    該当の行を直す。

関連
    :ref:`page-reference-aero-table-format`

.. _IY-A046:

IY-A046
-------

重大度: エラー

メッセージ::

    error[IY-A046]: bad row <line> in <path>: expected <n> values (as the header), found <m>

意味
    行の値の数が見出しの列数と合わない。

主な原因
    コンマの過不足。

対処
    該当の行を直す。

関連
    :ref:`page-reference-aero-table-format`

.. _IY-A047:

IY-A047
-------

重大度: エラー

メッセージ::

    error[IY-A047]: row <line> of <path>: column <k> (<name>) is not finite: <value>

意味
    行に NaN または無限大がある（座標の列も含む）。

主な原因
    計算の失敗で生じた NaN や inf。

対処
    値を有限の数にする。表を作った計算を確かめる。

関連
    :ref:`page-reference-aero-table-format`

.. _IY-A048:

IY-A048
-------

重大度: エラー

メッセージ::

    error[IY-A048]: row <line> of <path>: coordinates <found> do not match the grid point <expected> expected at this position (rows must follow the metadata axes in order: Mach outermost, then alpha[, then beta])

意味
    行の座標が、メタデータの軸から決まるその位置の格子点と一致しない。行は Mach 数を最も外側にした順に並ぶ。

主な原因
    行の順序の誤り、行の欠落や重複、メタデータの軸と CSV が一致していない。

対処
    メタデータの軸と CSV の座標を一致させ、行を所定の順に並べる。

関連
    :ref:`page-reference-aero-table-format`

.. _IY-A049:

IY-A049
-------

重大度: エラー

メッセージ::

    error[IY-A049]: aero table <path> has more rows than the <n> grid points of its metadata

意味
    CSV の行数がメタデータの格子点の数を超えている。

主な原因
    行の重複、メタデータの軸が短い。

対処
    行数をメタデータの軸の積に合わせる。

関連
    :ref:`page-reference-aero-table-format`

.. _IY-A050:

IY-A050
-------

重大度: エラー

メッセージ::

    error[IY-A050]: aero table <path> has <found> rows but its metadata declares <n> grid points

意味
    CSV の行数がメタデータの格子点の数に足りない。

主な原因
    行の欠落、書き出しの中断。

対処
    欠けた行を補うか、表を作り直す。

関連
    :ref:`page-reference-aero-table-format`

.. _IY-A501:

IY-A501
-------

重大度: ワーニング

メッセージ::

    warning[IY-A501]: aero table: <ca_on|ca_off> at Mach <m>, alpha 0 is <value> (negative axial force / thrust-like drag[; <k> Mach values in all])

意味
    外部の係数表で、迎角 0 の軸力係数 ``ca_on`` または ``ca_off`` が負である。推力のような抗力を意味し、物理的でない。列ごとに、最初の該当 Mach 数を示す。

主な原因
    符号規約の取り違え、表の作成誤り。

対処
    表の符号規約と値を確認する。

関連
    :ref:`page-reference-aero-table-format`

