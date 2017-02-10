package pt.adrz.clipx.gui;

/**
 * Singleton ... one and only one type of this object is allowed
 * @author adriano
 *
 */
public class ClipOptions {
	
	private static ClipOptions instance = null;

	private boolean enabled = true;
	
	protected ClipOptions() { }
	
	public static ClipOptions getInstance() {
		if ( ClipOptions.instance == null ) { ClipOptions.instance = new ClipOptions(); }
		return ClipOptions.instance;
	}
	
	public boolean state() { return enabled; }
	public void enable() { this.enabled = true; }
	public void disable() { this.enabled = false; }

}
