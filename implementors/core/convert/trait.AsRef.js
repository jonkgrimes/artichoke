(function() {var implementors = {};
implementors["artichoke_backend"] = [{"text":"impl&lt;'a&gt; AsRef&lt;Artichoke&gt; for Guard&lt;'a&gt;","synthetic":false,"types":[]},{"text":"impl&lt;'a, T&gt; AsRef&lt;T&gt; for UnboxedValueGuard&lt;'a, HeapAllocated&lt;T&gt;&gt;","synthetic":false,"types":[]},{"text":"impl AsRef&lt;Random&gt; for Random","synthetic":false,"types":[]},{"text":"impl&lt;'a&gt; AsRef&lt;Artichoke&gt; for ArenaIndex&lt;'a&gt;","synthetic":false,"types":[]}];
implementors["bstr"] = [{"text":"impl AsRef&lt;[u8]&gt; for BString","synthetic":false,"types":[]},{"text":"impl AsRef&lt;BStr&gt; for BString","synthetic":false,"types":[]},{"text":"impl AsRef&lt;[u8]&gt; for BStr","synthetic":false,"types":[]},{"text":"impl AsRef&lt;BStr&gt; for [u8]","synthetic":false,"types":[]},{"text":"impl AsRef&lt;BStr&gt; for str","synthetic":false,"types":[]}];
implementors["nix"] = [{"text":"impl AsRef&lt;str&gt; for Signal","synthetic":false,"types":[]},{"text":"impl AsRef&lt;sigset_t&gt; for SigSet","synthetic":false,"types":[]},{"text":"impl AsRef&lt;timespec&gt; for TimeSpec","synthetic":false,"types":[]},{"text":"impl AsRef&lt;timeval&gt; for TimeVal","synthetic":false,"types":[]}];
implementors["regex_syntax"] = [{"text":"impl AsRef&lt;[u8]&gt; for Literal","synthetic":false,"types":[]}];
implementors["smallvec"] = [{"text":"impl&lt;A:&nbsp;Array&gt; AsRef&lt;[&lt;A as Array&gt;::Item]&gt; for SmallVec&lt;A&gt;","synthetic":false,"types":[]}];
implementors["spinoso_array"] = [{"text":"impl&lt;T&gt; AsRef&lt;SmallVec&lt;[T; 8]&gt;&gt; for SmallArray&lt;T&gt;","synthetic":false,"types":[]},{"text":"impl&lt;T&gt; AsRef&lt;[T]&gt; for SmallArray&lt;T&gt;","synthetic":false,"types":[]},{"text":"impl&lt;T&gt; AsRef&lt;Vec&lt;T&gt;&gt; for Array&lt;T&gt;","synthetic":false,"types":[]},{"text":"impl&lt;T&gt; AsRef&lt;[T]&gt; for Array&lt;T&gt;","synthetic":false,"types":[]}];
if (window.register_implementors) {window.register_implementors(implementors);} else {window.pending_implementors = implementors;}})()