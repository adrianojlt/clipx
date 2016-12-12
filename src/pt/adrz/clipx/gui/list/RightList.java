package pt.adrz.clipx.gui.list;

import javax.swing.JList;
import javax.swing.JTextField;
import javax.swing.event.DocumentEvent;
import javax.swing.event.DocumentListener;

public class RightList extends JList<Node> implements DocumentListener {
	private static final long serialVersionUID = 1L;

	private JTextField search;
	private ClipListModel<Node> model;

	public RightList() {
		super();
		search = new JTextField(20);
		search.getDocument().addDocumentListener(this);
		model = new ClipListModel<Node>(search);
		this.setModel(model);
	}

	public ClipListModel<Node> getModel() {
		return this.model;
	}
	
	public JTextField getSearchField() {
		return this.search;
	}
	
	@Override
	public void insertUpdate(DocumentEvent e) {
		this.model.refilter();
	}

	@Override
	public void removeUpdate(DocumentEvent e) {
		this.model.refilter();
	}

	@Override
	public void changedUpdate(DocumentEvent e) {
		this.model.refilter();
	}
}
