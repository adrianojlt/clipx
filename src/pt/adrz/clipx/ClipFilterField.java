package pt.adrz.clipx;

import javax.swing.JTextField;
import javax.swing.event.DocumentEvent;
import javax.swing.event.DocumentListener;

public class ClipFilterField extends JTextField implements DocumentListener {
	
	private static final long serialVersionUID = 2406618860472659185L;
	
	// reference to list
	private ClipList list;
	
	
	public ClipFilterField(int width, ClipList list) {
		super(width);
		this.getDocument().addDocumentListener(this);
		this.list = list;
	}

	@Override
	public void changedUpdate(DocumentEvent arg0) {
		list.getModel().refilter();
	}

	@Override
	public void insertUpdate(DocumentEvent arg0) {
		list.getModel().refilter();
	}

	@Override
	public void removeUpdate(DocumentEvent arg0) {
		list.getModel().refilter();
	}

}
