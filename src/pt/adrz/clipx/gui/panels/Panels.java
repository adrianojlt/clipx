package pt.adrz.clipx.gui.panels;

import javax.swing.JTextArea;

public interface Panels {
	public void changeClipBoard(String text);
	public JTextArea getTextArea();
	public LeftPanel getLeftPanel();
	public RightPanel getRightPanel();
	public CentralPanel getCentralPanel();
}
