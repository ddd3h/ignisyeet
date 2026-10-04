IgnisYeet ドキュメント
======================

.. image:: _generated/logo/IgnisYeet-logo.*
   :width: 260px
   :align: center
   :alt: IgnisYeet logo

**IgnisYeet** は、ロケットの 3D モデル（STL ファイル）から空力特性を推算し、
6 自由度の飛翔シミュレーションと落下分散解析までを一貫して行うオープンソースの飛翔シミュレータである。

* STL を読み込み、機体の外形（胴体の断面形状とフィン）を自動で取り出す。
* 取り出した形状から、Mach 数と迎角の格子上で空力係数を **一度だけ** 計算して保存する。
* 保存した係数表を内挿・外挿しながら、ランチャ滑走から着地までを 6 自由度で数値積分する。
* 風速と風向を組み合わせた全ケースを並列に計算し、落下分散を求める。

計算の重い部分は Rust、作図は Python で実装している。

.. toctree::
   :maxdepth: 2
   :numbered:
   :caption: 目次

   intro
   usage
   overview
   coordinates
   geometry
   aerodynamics
   panel
   cfd
   environment
   propulsion
   dynamics
   dispersion
   verification
   limitations
