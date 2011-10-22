package pt.adrz.clipx;

import javax.swing.JList;
import javax.swing.JTextField;


public class ClipList extends JList{

	/**
	 * Field to search for String
	 */
	private ClipFilterField filterField;
	
	/**
	 * Model with String copied from clipboard
	 */
	private ClipFilterModel filterModel;
	
	private int FIELD_WIDTH = 20;
	
	public ClipList() {
		super();
		filterField = new ClipFilterField(FIELD_WIDTH, this);
		filterModel = new ClipFilterModel(filterField);
		this.setModel(filterModel);
	}
	
	
	
	public JTextField getFilterField() {
		return this.filterField;
	}
	
	
	public ClipFilterModel getModel() {
		return this.filterModel;
	}
	
	
	
}
