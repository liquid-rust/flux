(function() {var type_impls = {
"flux_middle":[["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-Binder%3CT%3E\" class=\"impl\"><a class=\"src rightside\" href=\"src/flux_middle/rustc/ty.rs.html#378-394\">source</a><a href=\"#impl-Binder%3CT%3E\" class=\"anchor\">§</a><h3 class=\"code-header\">impl&lt;T&gt; <a class=\"struct\" href=\"flux_middle/rustc/ty/struct.Binder.html\" title=\"struct flux_middle::rustc::ty::Binder\">Binder</a>&lt;T&gt;</h3></section></summary><div class=\"impl-items\"><section id=\"method.bind_with_vars\" class=\"method\"><a class=\"src rightside\" href=\"src/flux_middle/rustc/ty.rs.html#379-381\">source</a><h4 class=\"code-header\">pub fn <a href=\"flux_middle/rustc/ty/struct.Binder.html#tymethod.bind_with_vars\" class=\"fn\">bind_with_vars</a>(\n    value: T,\n    vars: impl <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/convert/trait.Into.html\" title=\"trait core::convert::Into\">Into</a>&lt;<a class=\"type\" href=\"flux_middle/intern/type.List.html\" title=\"type flux_middle::intern::List\">List</a>&lt;<a class=\"enum\" href=\"flux_middle/rustc/ty/enum.BoundVariableKind.html\" title=\"enum flux_middle::rustc::ty::BoundVariableKind\">BoundVariableKind</a>&gt;&gt;,\n) -&gt; <a class=\"struct\" href=\"flux_middle/rustc/ty/struct.Binder.html\" title=\"struct flux_middle::rustc::ty::Binder\">Binder</a>&lt;T&gt;</h4></section><section id=\"method.skip_binder\" class=\"method\"><a class=\"src rightside\" href=\"src/flux_middle/rustc/ty.rs.html#383-385\">source</a><h4 class=\"code-header\">pub fn <a href=\"flux_middle/rustc/ty/struct.Binder.html#tymethod.skip_binder\" class=\"fn\">skip_binder</a>(self) -&gt; T</h4></section><section id=\"method.as_ref\" class=\"method\"><a class=\"src rightside\" href=\"src/flux_middle/rustc/ty.rs.html#387-389\">source</a><h4 class=\"code-header\">pub fn <a href=\"flux_middle/rustc/ty/struct.Binder.html#tymethod.as_ref\" class=\"fn\">as_ref</a>(&amp;self) -&gt; <a class=\"struct\" href=\"flux_middle/rustc/ty/struct.Binder.html\" title=\"struct flux_middle::rustc::ty::Binder\">Binder</a>&lt;<a class=\"primitive\" href=\"https://doc.rust-lang.org/nightly/std/primitive.reference.html\">&amp;T</a>&gt;</h4></section><section id=\"method.vars\" class=\"method\"><a class=\"src rightside\" href=\"src/flux_middle/rustc/ty.rs.html#391-393\">source</a><h4 class=\"code-header\">pub fn <a href=\"flux_middle/rustc/ty/struct.Binder.html#tymethod.vars\" class=\"fn\">vars</a>(&amp;self) -&gt; &amp;<a class=\"type\" href=\"flux_middle/intern/type.List.html\" title=\"type flux_middle::intern::List\">List</a>&lt;<a class=\"enum\" href=\"flux_middle/rustc/ty/enum.BoundVariableKind.html\" title=\"enum flux_middle::rustc::ty::BoundVariableKind\">BoundVariableKind</a>&gt;</h4></section></div></details>",0,"flux_middle::rustc::ty::PolyFnSig"],["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-Clone-for-Binder%3CT%3E\" class=\"impl\"><a class=\"src rightside\" href=\"src/flux_middle/rustc/ty.rs.html#39\">source</a><a href=\"#impl-Clone-for-Binder%3CT%3E\" class=\"anchor\">§</a><h3 class=\"code-header\">impl&lt;T: <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/clone/trait.Clone.html\" title=\"trait core::clone::Clone\">Clone</a>&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/clone/trait.Clone.html\" title=\"trait core::clone::Clone\">Clone</a> for <a class=\"struct\" href=\"flux_middle/rustc/ty/struct.Binder.html\" title=\"struct flux_middle::rustc::ty::Binder\">Binder</a>&lt;T&gt;</h3></section></summary><div class=\"impl-items\"><details class=\"toggle method-toggle\" open><summary><section id=\"method.clone\" class=\"method trait-impl\"><a class=\"src rightside\" href=\"src/flux_middle/rustc/ty.rs.html#39\">source</a><a href=\"#method.clone\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"https://doc.rust-lang.org/nightly/core/clone/trait.Clone.html#tymethod.clone\" class=\"fn\">clone</a>(&amp;self) -&gt; <a class=\"struct\" href=\"flux_middle/rustc/ty/struct.Binder.html\" title=\"struct flux_middle::rustc::ty::Binder\">Binder</a>&lt;T&gt;</h4></section></summary><div class='docblock'>Returns a copy of the value. <a href=\"https://doc.rust-lang.org/nightly/core/clone/trait.Clone.html#tymethod.clone\">Read more</a></div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.clone_from\" class=\"method trait-impl\"><span class=\"rightside\"><span class=\"since\" title=\"Stable since Rust version 1.0.0\">1.0.0</span> · <a class=\"src\" href=\"https://doc.rust-lang.org/nightly/src/core/clone.rs.html#175\">source</a></span><a href=\"#method.clone_from\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"https://doc.rust-lang.org/nightly/core/clone/trait.Clone.html#method.clone_from\" class=\"fn\">clone_from</a>(&amp;mut self, source: &amp;Self)</h4></section></summary><div class='docblock'>Performs copy-assignment from <code>source</code>. <a href=\"https://doc.rust-lang.org/nightly/core/clone/trait.Clone.html#method.clone_from\">Read more</a></div></details></div></details>","Clone","flux_middle::rustc::ty::PolyFnSig"],["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-Debug-for-Binder%3CT%3E\" class=\"impl\"><a class=\"src rightside\" href=\"src/flux_middle/rustc/ty.rs.html#773-780\">source</a><a href=\"#impl-Debug-for-Binder%3CT%3E\" class=\"anchor\">§</a><h3 class=\"code-header\">impl&lt;T: <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/fmt/trait.Debug.html\" title=\"trait core::fmt::Debug\">Debug</a>&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/fmt/trait.Debug.html\" title=\"trait core::fmt::Debug\">Debug</a> for <a class=\"struct\" href=\"flux_middle/rustc/ty/struct.Binder.html\" title=\"struct flux_middle::rustc::ty::Binder\">Binder</a>&lt;T&gt;</h3></section></summary><div class=\"impl-items\"><details class=\"toggle method-toggle\" open><summary><section id=\"method.fmt\" class=\"method trait-impl\"><a class=\"src rightside\" href=\"src/flux_middle/rustc/ty.rs.html#774-779\">source</a><a href=\"#method.fmt\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"https://doc.rust-lang.org/nightly/core/fmt/trait.Debug.html#tymethod.fmt\" class=\"fn\">fmt</a>(&amp;self, f: &amp;mut <a class=\"struct\" href=\"https://doc.rust-lang.org/nightly/core/fmt/struct.Formatter.html\" title=\"struct core::fmt::Formatter\">Formatter</a>&lt;'_&gt;) -&gt; <a class=\"type\" href=\"https://doc.rust-lang.org/nightly/core/fmt/type.Result.html\" title=\"type core::fmt::Result\">Result</a></h4></section></summary><div class='docblock'>Formats the value using the given formatter. <a href=\"https://doc.rust-lang.org/nightly/core/fmt/trait.Debug.html#tymethod.fmt\">Read more</a></div></details></div></details>","Debug","flux_middle::rustc::ty::PolyFnSig"],["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-Decodable%3C__D%3E-for-Binder%3CT%3E\" class=\"impl\"><a class=\"src rightside\" href=\"src/flux_middle/rustc/ty.rs.html#39\">source</a><a href=\"#impl-Decodable%3C__D%3E-for-Binder%3CT%3E\" class=\"anchor\">§</a><h3 class=\"code-header\">impl&lt;T, __D: TyDecoder&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/nightly-rustc/rustc_serialize/serialize/trait.Decodable.html\" title=\"trait rustc_serialize::serialize::Decodable\">Decodable</a>&lt;__D&gt; for <a class=\"struct\" href=\"flux_middle/rustc/ty/struct.Binder.html\" title=\"struct flux_middle::rustc::ty::Binder\">Binder</a>&lt;T&gt;<div class=\"where\">where\n    T: <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/nightly-rustc/rustc_serialize/serialize/trait.Decodable.html\" title=\"trait rustc_serialize::serialize::Decodable\">Decodable</a>&lt;__D&gt;,</div></h3></section></summary><div class=\"impl-items\"><section id=\"method.decode\" class=\"method trait-impl\"><a class=\"src rightside\" href=\"src/flux_middle/rustc/ty.rs.html#39\">source</a><a href=\"#method.decode\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"https://doc.rust-lang.org/nightly/nightly-rustc/rustc_serialize/serialize/trait.Decodable.html#tymethod.decode\" class=\"fn\">decode</a>(__decoder: <a class=\"primitive\" href=\"https://doc.rust-lang.org/nightly/std/primitive.reference.html\">&amp;mut __D</a>) -&gt; Self</h4></section></div></details>","Decodable<__D>","flux_middle::rustc::ty::PolyFnSig"],["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-Encodable%3C__E%3E-for-Binder%3CT%3E\" class=\"impl\"><a class=\"src rightside\" href=\"src/flux_middle/rustc/ty.rs.html#39\">source</a><a href=\"#impl-Encodable%3C__E%3E-for-Binder%3CT%3E\" class=\"anchor\">§</a><h3 class=\"code-header\">impl&lt;T, __E: TyEncoder&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/nightly-rustc/rustc_serialize/serialize/trait.Encodable.html\" title=\"trait rustc_serialize::serialize::Encodable\">Encodable</a>&lt;__E&gt; for <a class=\"struct\" href=\"flux_middle/rustc/ty/struct.Binder.html\" title=\"struct flux_middle::rustc::ty::Binder\">Binder</a>&lt;T&gt;<div class=\"where\">where\n    T: <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/nightly-rustc/rustc_serialize/serialize/trait.Encodable.html\" title=\"trait rustc_serialize::serialize::Encodable\">Encodable</a>&lt;__E&gt;,</div></h3></section></summary><div class=\"impl-items\"><section id=\"method.encode\" class=\"method trait-impl\"><a class=\"src rightside\" href=\"src/flux_middle/rustc/ty.rs.html#39\">source</a><a href=\"#method.encode\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"https://doc.rust-lang.org/nightly/nightly-rustc/rustc_serialize/serialize/trait.Encodable.html#tymethod.encode\" class=\"fn\">encode</a>(&amp;self, __encoder: <a class=\"primitive\" href=\"https://doc.rust-lang.org/nightly/std/primitive.reference.html\">&amp;mut __E</a>)</h4></section></div></details>","Encodable<__E>","flux_middle::rustc::ty::PolyFnSig"],["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-Hash-for-Binder%3CT%3E\" class=\"impl\"><a class=\"src rightside\" href=\"src/flux_middle/rustc/ty.rs.html#39\">source</a><a href=\"#impl-Hash-for-Binder%3CT%3E\" class=\"anchor\">§</a><h3 class=\"code-header\">impl&lt;T: <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/hash/trait.Hash.html\" title=\"trait core::hash::Hash\">Hash</a>&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/hash/trait.Hash.html\" title=\"trait core::hash::Hash\">Hash</a> for <a class=\"struct\" href=\"flux_middle/rustc/ty/struct.Binder.html\" title=\"struct flux_middle::rustc::ty::Binder\">Binder</a>&lt;T&gt;</h3></section></summary><div class=\"impl-items\"><details class=\"toggle method-toggle\" open><summary><section id=\"method.hash\" class=\"method trait-impl\"><a class=\"src rightside\" href=\"src/flux_middle/rustc/ty.rs.html#39\">source</a><a href=\"#method.hash\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"https://doc.rust-lang.org/nightly/core/hash/trait.Hash.html#tymethod.hash\" class=\"fn\">hash</a>&lt;__H: <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/hash/trait.Hasher.html\" title=\"trait core::hash::Hasher\">Hasher</a>&gt;(&amp;self, state: <a class=\"primitive\" href=\"https://doc.rust-lang.org/nightly/std/primitive.reference.html\">&amp;mut __H</a>)</h4></section></summary><div class='docblock'>Feeds this value into the given <a href=\"https://doc.rust-lang.org/nightly/core/hash/trait.Hasher.html\" title=\"trait core::hash::Hasher\"><code>Hasher</code></a>. <a href=\"https://doc.rust-lang.org/nightly/core/hash/trait.Hash.html#tymethod.hash\">Read more</a></div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.hash_slice\" class=\"method trait-impl\"><span class=\"rightside\"><span class=\"since\" title=\"Stable since Rust version 1.3.0\">1.3.0</span> · <a class=\"src\" href=\"https://doc.rust-lang.org/nightly/src/core/hash/mod.rs.html#235-237\">source</a></span><a href=\"#method.hash_slice\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"https://doc.rust-lang.org/nightly/core/hash/trait.Hash.html#method.hash_slice\" class=\"fn\">hash_slice</a>&lt;H&gt;(data: &amp;[Self], state: <a class=\"primitive\" href=\"https://doc.rust-lang.org/nightly/std/primitive.reference.html\">&amp;mut H</a>)<div class=\"where\">where\n    H: <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/hash/trait.Hasher.html\" title=\"trait core::hash::Hasher\">Hasher</a>,\n    Self: <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/marker/trait.Sized.html\" title=\"trait core::marker::Sized\">Sized</a>,</div></h4></section></summary><div class='docblock'>Feeds a slice of this type into the given <a href=\"https://doc.rust-lang.org/nightly/core/hash/trait.Hasher.html\" title=\"trait core::hash::Hasher\"><code>Hasher</code></a>. <a href=\"https://doc.rust-lang.org/nightly/core/hash/trait.Hash.html#method.hash_slice\">Read more</a></div></details></div></details>","Hash","flux_middle::rustc::ty::PolyFnSig"],["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-PartialEq-for-Binder%3CT%3E\" class=\"impl\"><a class=\"src rightside\" href=\"src/flux_middle/rustc/ty.rs.html#39\">source</a><a href=\"#impl-PartialEq-for-Binder%3CT%3E\" class=\"anchor\">§</a><h3 class=\"code-header\">impl&lt;T: <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/cmp/trait.PartialEq.html\" title=\"trait core::cmp::PartialEq\">PartialEq</a>&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/cmp/trait.PartialEq.html\" title=\"trait core::cmp::PartialEq\">PartialEq</a> for <a class=\"struct\" href=\"flux_middle/rustc/ty/struct.Binder.html\" title=\"struct flux_middle::rustc::ty::Binder\">Binder</a>&lt;T&gt;</h3></section></summary><div class=\"impl-items\"><details class=\"toggle method-toggle\" open><summary><section id=\"method.eq\" class=\"method trait-impl\"><a class=\"src rightside\" href=\"src/flux_middle/rustc/ty.rs.html#39\">source</a><a href=\"#method.eq\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"https://doc.rust-lang.org/nightly/core/cmp/trait.PartialEq.html#tymethod.eq\" class=\"fn\">eq</a>(&amp;self, other: &amp;<a class=\"struct\" href=\"flux_middle/rustc/ty/struct.Binder.html\" title=\"struct flux_middle::rustc::ty::Binder\">Binder</a>&lt;T&gt;) -&gt; <a class=\"primitive\" href=\"https://doc.rust-lang.org/nightly/std/primitive.bool.html\">bool</a></h4></section></summary><div class='docblock'>Tests for <code>self</code> and <code>other</code> values to be equal, and is used by <code>==</code>.</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.ne\" class=\"method trait-impl\"><span class=\"rightside\"><span class=\"since\" title=\"Stable since Rust version 1.0.0\">1.0.0</span> · <a class=\"src\" href=\"https://doc.rust-lang.org/nightly/src/core/cmp.rs.html#261\">source</a></span><a href=\"#method.ne\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"https://doc.rust-lang.org/nightly/core/cmp/trait.PartialEq.html#method.ne\" class=\"fn\">ne</a>(&amp;self, other: <a class=\"primitive\" href=\"https://doc.rust-lang.org/nightly/std/primitive.reference.html\">&amp;Rhs</a>) -&gt; <a class=\"primitive\" href=\"https://doc.rust-lang.org/nightly/std/primitive.bool.html\">bool</a></h4></section></summary><div class='docblock'>Tests for <code>!=</code>. The default implementation is almost always sufficient,\nand should not be overridden without very good reason.</div></details></div></details>","PartialEq","flux_middle::rustc::ty::PolyFnSig"],["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-Subst-for-Binder%3CT%3E\" class=\"impl\"><a class=\"src rightside\" href=\"src/flux_middle/rustc/ty/subst.rs.html#14-21\">source</a><a href=\"#impl-Subst-for-Binder%3CT%3E\" class=\"anchor\">§</a><h3 class=\"code-header\">impl&lt;T&gt; <a class=\"trait\" href=\"flux_middle/rustc/ty/subst/trait.Subst.html\" title=\"trait flux_middle::rustc::ty::subst::Subst\">Subst</a> for <a class=\"struct\" href=\"flux_middle/rustc/ty/struct.Binder.html\" title=\"struct flux_middle::rustc::ty::Binder\">Binder</a>&lt;T&gt;<div class=\"where\">where\n    T: <a class=\"trait\" href=\"flux_middle/rustc/ty/subst/trait.Subst.html\" title=\"trait flux_middle::rustc::ty::subst::Subst\">Subst</a>,</div></h3></section></summary><div class=\"impl-items\"><section id=\"method.subst\" class=\"method trait-impl\"><a class=\"src rightside\" href=\"src/flux_middle/rustc/ty/subst.rs.html#18-20\">source</a><a href=\"#method.subst\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"flux_middle/rustc/ty/subst/trait.Subst.html#tymethod.subst\" class=\"fn\">subst</a>(&amp;self, args: &amp;[<a class=\"enum\" href=\"flux_middle/rustc/ty/enum.GenericArg.html\" title=\"enum flux_middle::rustc::ty::GenericArg\">GenericArg</a>]) -&gt; Self</h4></section></div></details>","Subst","flux_middle::rustc::ty::PolyFnSig"],["<section id=\"impl-Eq-for-Binder%3CT%3E\" class=\"impl\"><a class=\"src rightside\" href=\"src/flux_middle/rustc/ty.rs.html#39\">source</a><a href=\"#impl-Eq-for-Binder%3CT%3E\" class=\"anchor\">§</a><h3 class=\"code-header\">impl&lt;T: <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/cmp/trait.Eq.html\" title=\"trait core::cmp::Eq\">Eq</a>&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/cmp/trait.Eq.html\" title=\"trait core::cmp::Eq\">Eq</a> for <a class=\"struct\" href=\"flux_middle/rustc/ty/struct.Binder.html\" title=\"struct flux_middle::rustc::ty::Binder\">Binder</a>&lt;T&gt;</h3></section>","Eq","flux_middle::rustc::ty::PolyFnSig"],["<section id=\"impl-StructuralPartialEq-for-Binder%3CT%3E\" class=\"impl\"><a class=\"src rightside\" href=\"src/flux_middle/rustc/ty.rs.html#39\">source</a><a href=\"#impl-StructuralPartialEq-for-Binder%3CT%3E\" class=\"anchor\">§</a><h3 class=\"code-header\">impl&lt;T&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/marker/trait.StructuralPartialEq.html\" title=\"trait core::marker::StructuralPartialEq\">StructuralPartialEq</a> for <a class=\"struct\" href=\"flux_middle/rustc/ty/struct.Binder.html\" title=\"struct flux_middle::rustc::ty::Binder\">Binder</a>&lt;T&gt;</h3></section>","StructuralPartialEq","flux_middle::rustc::ty::PolyFnSig"]]
};if (window.register_type_impls) {window.register_type_impls(type_impls);} else {window.pending_type_impls = type_impls;}})()