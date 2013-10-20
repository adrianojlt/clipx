package pt.adrz.clipx;

public class ClipOptions {

	private boolean enabled = true;
	
	public ClipOptions() {
	}
	
	public boolean isEnabled() { return enabled; }
	public void enable() { this.enabled = true; }
	public void disable() { this.enabled = false; }

}
