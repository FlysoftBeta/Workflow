import java.util.*;
import java.lang.reflect.*;
import top.flysoftbeta.workflow.core.layout.*;
import top.flysoftbeta.workflow.core.store.StateCodec;
import top.flysoftbeta.workflow.core.json.Json;
/** Uses the frozen Kotlin reducer as an independent differential oracle. Output is JSONL. */
public class LayoutOracle {
 static Object wire(Object x) throws Exception {
  if(x==null || x instanceof String || x instanceof Number || x instanceof Boolean) return x;
  if(x instanceof Enum<?> e) return e.name().toLowerCase(Locale.ROOT);
  if(x instanceof PanelTarget t) return StateCodec.INSTANCE.target(t);
  if(x instanceof PanelView v) return StateCodec.INSTANCE.view(v);
  if(x instanceof List<?> a) {var out=new ArrayList<Object>(); for(var v:a)out.add(wire(v));return out;}
  if(x instanceof Map<?,?> m) {var out=new LinkedHashMap<String,Object>();for(var e:m.entrySet())out.put(e.getKey().toString(),wire(e.getValue()));return out;}
  var out=new LinkedHashMap<String,Object>();
  if(x instanceof LayoutOp || x instanceof Placement || x instanceof DropTarget) {String n=x.getClass().getSimpleName();out.put("type",Character.toLowerCase(n.charAt(0))+n.substring(1));}
  for(Field f:x.getClass().getDeclaredFields()) if(!Modifier.isStatic(f.getModifiers())) {f.setAccessible(true);out.put(f.getName(),wire(f.get(x)));}
  return out;
 }
 public static void main(String[] args) throws Exception {
  int count=args.length==0?400:Integer.parseInt(args[0]);
  for(int seed=0;seed<count;seed++){
   var random=kotlin.random.RandomKt.Random(seed);var w=Workbench.Companion.empty();
   for(int step=0;step<80;step++){
    var op=LayoutPropertyTest.Companion.randomOp(w,random);var after=w.apply(op);
    if(!after.violations().isEmpty())throw new AssertionError(after.violations());
    var record=new LinkedHashMap<String,Object>();record.put("seed",seed);record.put("step",step);record.put("before",StateCodec.INSTANCE.workbench(w));record.put("op",wire(op));record.put("after",StateCodec.INSTANCE.workbench(after));
    System.out.println(Json.INSTANCE.stringify(record));w=after;
   }
  }
 }
}
