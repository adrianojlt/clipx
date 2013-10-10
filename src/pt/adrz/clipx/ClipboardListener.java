package pt.adrz.clipx;

public interface ClipboardListener {

	public void newString(String copyString);
	public ClipList getList();
}
